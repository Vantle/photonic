use std::collections::HashMap;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub const ADDRESS: &str = "127.0.0.1:8080";
const HOST: [&str; 2] = [ADDRESS, "localhost:8080"];
const TEXT: &str = "text/plain; charset=utf-8";
const LIMIT: u64 = 512 * 1024;
const READ: Duration = Duration::from_secs(2);
const WRITE: Duration = Duration::from_secs(10);

#[derive(Debug, Eq, PartialEq)]
pub enum Route<'book> {
    File {
        path: &'book PathBuf,
        kind: &'static str,
    },
    Moved(String),
    Forbidden,
    Method,
    Long,
    Large,
}

fn media(name: &str) -> &'static str {
    match name.rsplit_once('.').map(|(_, extension)| extension) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("svg") => "image/svg+xml",
        _ => TEXT,
    }
}

impl<'book> Route<'book> {
    pub fn new(request: &str, book: &'book HashMap<String, PathBuf>, repository: &str) -> Self {
        let mut line = request.lines();
        let mut start = line.next().unwrap_or_default().split_whitespace();
        let method = start.next();
        let target = start.next().unwrap_or("/");
        let host = line
            .take_while(|header| !header.is_empty())
            .find_map(|header| {
                let (field, value) = header.split_once(':')?;
                field
                    .trim()
                    .eq_ignore_ascii_case("host")
                    .then(|| value.trim())
            });
        if !host.is_some_and(|value| HOST.contains(&value)) {
            return Self::Forbidden;
        }
        if method != Some("GET") {
            return Self::Method;
        }
        let resource = target
            .split('?')
            .next()
            .unwrap_or_default()
            .trim_start_matches('/');
        let name = if resource.is_empty() {
            "index.html"
        } else {
            resource
        };
        match book.get(name) {
            Some(path) => Self::File {
                path,
                kind: media(name),
            },
            None => {
                // The published site points every repository link at GitHub, so a served checkout sends them there too.
                let view = if name.ends_with('/') { "tree" } else { "blob" };
                Self::Moved(format!("{repository}/{view}/main/{name}"))
            }
        }
    }

    pub fn status(&self) -> &'static str {
        match self {
            Self::File { .. } => "200 OK",
            Self::Moved(_) => "302 Found",
            Self::Forbidden => "403 Forbidden",
            Self::Method => "405 Method Not Allowed",
            Self::Long => "414 URI Too Long",
            Self::Large => "431 Request Header Fields Too Large",
        }
    }

    pub fn header(&self) -> String {
        match self {
            Self::Moved(location) => format!("Location: {location}\r\n"),
            Self::Method => "Allow: GET\r\n".to_owned(),
            Self::File { .. } | Self::Forbidden | Self::Long | Self::Large => String::new(),
        }
    }
}

struct Reply {
    status: &'static str,
    kind: &'static str,
    header: String,
    body: Vec<u8>,
}

impl Reply {
    fn new(route: &Route<'_>) -> Self {
        let text = |body: String| Self {
            status: route.status(),
            kind: TEXT,
            header: route.header(),
            body: body.into_bytes(),
        };
        let limit = LIMIT / 1024;
        match route {
            Route::File { path, kind } => match std::fs::read(path) {
                Ok(body) => Self {
                    status: route.status(),
                    kind: *kind,
                    header: route.header(),
                    body,
                },
                Err(error) => Self {
                    status: "500 Internal Server Error",
                    kind: TEXT,
                    header: String::new(),
                    body: format!("The server could not read {}: {error}", path.display())
                        .into_bytes(),
                },
            },
            Route::Moved(location) => text(format!("This file is in the repository: {location}")),
            Route::Forbidden => text(format!("Open the book at http://{ADDRESS}.")),
            Route::Method => text("This server answers only GET requests.".to_owned()),
            Route::Long => text(format!(
                "This link is longer than the {limit} KiB the server reads."
            )),
            Route::Large => text(format!(
                "These request headers are larger than the {limit} KiB the server reads."
            )),
        }
    }

    fn send(&self, mut stream: &TcpStream) -> std::io::Result<()> {
        let head = format!(
            "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n{}Cache-Control: no-cache\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n",
            self.status,
            self.kind,
            self.body.len(),
            self.header
        );
        stream.write_all(head.as_bytes())?;
        stream.write_all(&self.body)
    }
}

fn request<'book>(
    stream: &TcpStream,
    book: &'book HashMap<String, PathBuf>,
    repository: &str,
) -> std::io::Result<Route<'book>> {
    let mut reader = BufReader::new(stream).take(LIMIT);
    let mut head = Vec::new();
    loop {
        let start = head.len();
        reader.read_until(b'\n', &mut head)?;
        let line = &head[start..];
        if matches!(line, b"\r\n" | b"\n") {
            return Ok(Route::new(
                &String::from_utf8_lossy(&head),
                book,
                repository,
            ));
        }
        if line.ends_with(b"\n") {
            continue;
        }
        if reader.limit() > 0 {
            return Err(ErrorKind::UnexpectedEof.into());
        }
        return Ok(if start == 0 {
            Route::Long
        } else {
            Route::Large
        });
    }
}

fn answer(
    stream: &TcpStream,
    book: &HashMap<String, PathBuf>,
    repository: &str,
) -> std::io::Result<()> {
    stream.set_read_timeout(Some(READ))?;
    stream.set_write_timeout(Some(WRITE))?;
    let route = request(stream, book, repository)?;
    Reply::new(&route).send(stream)?;
    // Closing a socket that still holds unread input resets the connection, and a reset can discard the reply before the client reads it.
    stream.shutdown(Shutdown::Write)?;
    std::io::copy(&mut stream.take(LIMIT), &mut std::io::sink())?;
    Ok(())
}

pub fn serve(listener: TcpListener, book: HashMap<String, PathBuf>, repository: &'static str) {
    let book = Arc::new(book);
    for connection in listener.incoming() {
        let stream = match connection {
            Ok(stream) => stream,
            Err(error) => {
                eprintln!("The server could not accept a connection: {error}");
                continue;
            }
        };
        let book = Arc::clone(&book);
        let thread = std::thread::Builder::new().spawn(move || {
            // Browsers open connections they never use; their reads time out and end only their own thread.
            let _ = answer(&stream, &book, repository);
        });
        if let Err(error) = thread {
            eprintln!("The server could not answer a connection: {error}");
        }
    }
}

#[cfg(test)]
#[path = "test/route.rs"]
mod test;
