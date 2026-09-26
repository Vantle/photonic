use super::{Route, serve};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::time::Duration;

const REPOSITORY: &str = "https://github.com/Vantle/photonic";
const WASM: &str = "toolchain/browser/module/runtime_bg.wasm";

fn book() -> HashMap<String, PathBuf> {
    ["index.html", "lightbox.html", "book/book.css", WASM]
        .into_iter()
        .map(|name| (name.to_owned(), PathBuf::from("/runfile").join(name)))
        .collect()
}

fn get(target: &str, host: &str) -> String {
    format!("GET {target} HTTP/1.1\r\nHost: {host}\r\nAccept: */*\r\n\r\n")
}

fn server(name: &str, file: &[(&str, &[u8])]) -> SocketAddr {
    let directory =
        PathBuf::from(std::env::var_os("TEST_TMPDIR").expect("a test directory")).join(name);
    let book = file
        .iter()
        .map(|(path, content)| {
            let location = directory.join(path);
            std::fs::create_dir_all(location.parent().expect("a parent directory"))
                .expect("a book directory");
            std::fs::write(&location, content).expect("a book file");
            ((*path).to_owned(), location)
        })
        .collect();
    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
    let address = listener.local_addr().expect("a local address");
    std::thread::spawn(move || serve(listener, book, REPOSITORY));
    address
}

fn connect(address: SocketAddr) -> TcpStream {
    let stream = TcpStream::connect(address).expect("a connection");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("a read timeout");
    stream
}

fn reply(mut stream: TcpStream) -> String {
    let mut buffer = Vec::new();
    stream.read_to_end(&mut buffer).expect("a reply");
    String::from_utf8_lossy(&buffer).into_owned()
}

fn exchange(address: SocketAddr, request: &str) -> String {
    let mut stream = connect(address);
    stream.write_all(request.as_bytes()).expect("a request");
    reply(stream)
}

fn status(text: &str) -> &str {
    text.lines().next().unwrap_or_default()
}

#[test]
fn file() {
    let book = book();
    let page = Route::new(&get("/", "127.0.0.1:8080"), &book, REPOSITORY);
    assert_eq!(
        page,
        Route::File {
            path: &book["index.html"],
            kind: "text/html; charset=utf-8"
        }
    );
    assert_eq!(page.status(), "200 OK");
    assert_eq!(page.header(), "");
    assert_eq!(
        Route::new(
            &get("/lightbox.html?source=A%2C+%5BA%5D+B", "localhost:8080"),
            &book,
            REPOSITORY
        ),
        Route::File {
            path: &book["lightbox.html"],
            kind: "text/html; charset=utf-8"
        }
    );
    assert_eq!(
        Route::new(&get("/book/book.css", "localhost:8080"), &book, REPOSITORY),
        Route::File {
            path: &book["book/book.css"],
            kind: "text/css; charset=utf-8"
        }
    );
    assert_eq!(
        Route::new(
            &get(&format!("/{WASM}"), "127.0.0.1:8080"),
            &book,
            REPOSITORY
        ),
        Route::File {
            path: &book[WASM],
            kind: "application/wasm"
        }
    );
}

#[test]
fn host() {
    let book = book();
    assert_eq!(
        Route::new("GET / HTTP/1.1\r\n\r\n", &book, REPOSITORY),
        Route::Forbidden
    );
    assert_eq!(
        Route::new(&get("/", "example.com:8080"), &book, REPOSITORY),
        Route::Forbidden
    );
    assert_eq!(
        Route::new(
            "GET / HTTP/1.1\r\nAccept: */*\r\n\r\nHost: 127.0.0.1:8080\r\n",
            &book,
            REPOSITORY
        ),
        Route::Forbidden
    );
    assert_eq!(
        Route::new(
            "GET / HTTP/1.1\r\nhost:  localhost:8080 \r\n\r\n",
            &book,
            REPOSITORY
        ),
        Route::File {
            path: &book["index.html"],
            kind: "text/html; charset=utf-8"
        }
    );
    assert_eq!(Route::Forbidden.status(), "403 Forbidden");
    assert_eq!(Route::Forbidden.header(), "");
}

#[test]
fn method() {
    let book = book();
    let refused = Route::new(
        "POST / HTTP/1.1\r\nHost: 127.0.0.1:8080\r\n\r\n",
        &book,
        REPOSITORY,
    );
    assert_eq!(refused, Route::Method);
    assert_eq!(refused.status(), "405 Method Not Allowed");
    assert_eq!(refused.header(), "Allow: GET\r\n");
}

#[test]
fn repository() {
    let book = book();
    let document = Route::new(
        &get("/document/build.md", "127.0.0.1:8080"),
        &book,
        REPOSITORY,
    );
    assert_eq!(
        document,
        Route::Moved(format!("{REPOSITORY}/blob/main/document/build.md"))
    );
    assert_eq!(document.status(), "302 Found");
    assert_eq!(
        document.header(),
        format!("Location: {REPOSITORY}/blob/main/document/build.md\r\n")
    );
    assert_eq!(
        Route::new(&get("/library/", "localhost:8080"), &book, REPOSITORY),
        Route::Moved(format!("{REPOSITORY}/tree/main/library/"))
    );
}

#[test]
fn split() {
    let address = server("split", &[("index.html", b"<title>Photonic</title>")]);
    let mut stream = connect(address);
    for part in ["GET / HTTP/1.1\r\n", "Host: 127.0.0.1:8080\r\n", "\r\n"] {
        stream.write_all(part.as_bytes()).expect("a request part");
        std::thread::sleep(Duration::from_millis(50));
    }
    let text = reply(stream);
    assert_eq!(status(&text), "HTTP/1.1 200 OK");
    assert!(text.ends_with("\r\n\r\n<title>Photonic</title>"), "{text}");
}

#[test]
fn size() {
    let address = server("size", &[("lightbox.html", b"<title>Lightbox</title>")]);
    let link = format!("/lightbox.html?source={}", "A%2C".repeat(100 * 1024));
    let text = exchange(address, &get(&link, "localhost:8080"));
    assert_eq!(status(&text), "HTTP/1.1 200 OK");
    assert!(text.ends_with("\r\n\r\n<title>Lightbox</title>"));
    let link = format!("/lightbox.html?source={}", "A".repeat(600 * 1024));
    let text = exchange(address, &get(&link, "localhost:8080"));
    assert_eq!(status(&text), "HTTP/1.1 414 URI Too Long");
    let cookie = "A".repeat(600 * 1024);
    let text = exchange(
        address,
        &format!("GET / HTTP/1.1\r\nHost: localhost:8080\r\nCookie: {cookie}\r\n\r\n"),
    );
    assert_eq!(
        status(&text),
        "HTTP/1.1 431 Request Header Fields Too Large"
    );
}

#[test]
fn stall() {
    let module = vec![0; 32 * 1024 * 1024];
    let address = server(
        "stall",
        &[("index.html", b"<title>Photonic</title>"), (WASM, &module)],
    );
    let mut stalled = connect(address);
    stalled
        .write_all(get(&format!("/{WASM}"), "127.0.0.1:8080").as_bytes())
        .expect("a request");
    let mut idle = connect(address);
    let text = exchange(address, &get("/", "127.0.0.1:8080"));
    assert_eq!(status(&text), "HTTP/1.1 200 OK");
    idle.write_all(get("/", "localhost:8080").as_bytes())
        .expect("a late request");
    assert_eq!(status(&reply(idle)), "HTTP/1.1 200 OK");
    let mut start = [0; 15];
    stalled
        .read_exact(&mut start)
        .expect("the start of a reply");
    assert_eq!(&start, b"HTTP/1.1 200 OK");
}
