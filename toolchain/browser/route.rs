use std::collections::HashMap;
use std::path::PathBuf;

const HOST: [&str; 2] = ["127.0.0.1:8080", "localhost:8080"];

#[derive(Debug, Eq, PartialEq)]
pub enum Route<'book> {
    File {
        path: &'book PathBuf,
        kind: &'static str,
    },
    Moved(String),
    Forbidden,
    Method,
}

fn media(name: &str) -> &'static str {
    match name.rsplit_once('.').map(|(_, extension)| extension) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("svg") => "image/svg+xml",
        _ => "text/plain; charset=utf-8",
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
        }
    }

    pub fn header(&self) -> String {
        match self {
            Self::Moved(location) => format!("Location: {location}\r\n"),
            Self::Method => "Allow: GET\r\n".to_owned(),
            Self::File { .. } | Self::Forbidden => String::new(),
        }
    }
}

#[cfg(test)]
#[path = "test/route.rs"]
mod test;
