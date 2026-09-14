use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Component, Path, PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(
        std::env::var_os("BUILD_WORKSPACE_DIRECTORY")
            .ok_or("run this preview with bazel run //browser:serve")?,
    );
    let runfile = runfiles::Runfiles::create()?;
    let javascript = runfile
        .rlocation_from(&std::env::var("JAVASCRIPT")?, "")
        .ok_or("missing JavaScript binding")?;
    let webassembly = runfile
        .rlocation_from(&std::env::var("WEBASSEMBLY")?, "")
        .ok_or("missing WebAssembly module")?;
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("Photonic webbook: http://127.0.0.1:8080");
    for stream in listener.incoming() {
        let mut stream = stream?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
        let mut buffer = [0; 8192];
        let count = match stream.read(&mut buffer) {
            Ok(count) => count,
            Err(_) => continue,
        };
        let request = String::from_utf8_lossy(&buffer[..count]);
        let mut line = request
            .lines()
            .next()
            .unwrap_or_default()
            .split_whitespace();
        let method = line.next();
        let path = line.next().unwrap_or("/").split('?').next().unwrap_or("/");
        let relative = path.strip_prefix('/').unwrap_or_default();
        let relative = if relative.is_empty() {
            "index.html"
        } else {
            relative
        };
        let safe = method == Some("GET")
            && Path::new(relative)
                .components()
                .all(|part| matches!(part, Component::Normal(_)));
        let path = match relative {
            "browser/module/runtime.js" => javascript.clone(),
            "browser/module/runtime_bg.wasm" => webassembly.clone(),
            _ => root.join(relative),
        };
        let content = if safe {
            std::fs::read(&path).ok()
        } else {
            None
        };
        let (status, body) = content
            .map(|body| ("200 OK", body))
            .unwrap_or_else(|| ("404 Not Found", b"Not found".to_vec()));
        let kind = match path.extension().and_then(|value| value.to_str()) {
            Some("html") => "text/html; charset=utf-8",
            Some("js") => "text/javascript; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("wasm") => "application/wasm",
            _ => "text/plain; charset=utf-8",
        };
        let _ = write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(&body);
    }
    Ok(())
}
