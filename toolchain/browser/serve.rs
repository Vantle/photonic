mod route;

use route::Route;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

const ADDRESS: &str = "127.0.0.1:8080";
const TEXT: &str = "text/plain; charset=utf-8";

struct Reply {
    status: &'static str,
    kind: &'static str,
    body: Vec<u8>,
}

impl Reply {
    fn new(route: &Route<'_>) -> Self {
        let text = |body: String| Self {
            status: route.status(),
            kind: TEXT,
            body: body.into_bytes(),
        };
        match route {
            Route::File { path, kind } => match std::fs::read(path) {
                Ok(body) => Self {
                    status: route.status(),
                    kind: *kind,
                    body,
                },
                Err(error) => Self {
                    status: "500 Internal Server Error",
                    kind: TEXT,
                    body: format!("The server could not read {}: {error}", path.display())
                        .into_bytes(),
                },
            },
            Route::Moved(location) => text(format!("This file is in the repository: {location}")),
            Route::Forbidden => text(format!("Open the book at http://{ADDRESS}.")),
            Route::Method => text("This server answers only GET requests.".to_owned()),
        }
    }
}

fn catalog() -> Result<HashMap<String, PathBuf>, Box<dyn std::error::Error>> {
    let runfile = runfiles::Runfiles::create()?;
    std::env::var("BOOK")?
        .split_whitespace()
        .map(|location| {
            let (_, path) = location
                .split_once('/')
                .ok_or("a book file has no repository")?;
            let file = runfile
                .rlocation_from(location, "")
                .ok_or("a book file is missing from the runfiles")?;
            Ok((path.to_owned(), file))
        })
        .collect()
}

fn answer(stream: &mut TcpStream, book: &HashMap<String, PathBuf>) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    let mut buffer = [0; 8192];
    let count = stream.read(&mut buffer)?;
    let route = Route::new(
        &String::from_utf8_lossy(&buffer[..count]),
        book,
        env!("REPOSITORY"),
    );
    let reply = Reply::new(&route);
    write!(
        stream,
        "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n{}Cache-Control: no-cache\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n",
        reply.status,
        reply.kind,
        reply.body.len(),
        route.header()
    )?;
    stream.write_all(&reply.body)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let book = catalog()?;
    let listener = TcpListener::bind(ADDRESS)?;
    println!("Photonic webbook: http://{ADDRESS}");
    for connection in listener.incoming() {
        match connection {
            Ok(mut stream) => {
                // Browsers open connections they never use; those reads time out, and the server moves on.
                let _ = answer(&mut stream, &book);
            }
            Err(error) => eprintln!("The server could not accept a connection: {error}"),
        }
    }
    Ok(())
}
