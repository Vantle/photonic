use super::Route;
use std::collections::HashMap;
use std::path::PathBuf;

const REPOSITORY: &str = "https://github.com/Vantle/photonic";

fn book() -> HashMap<String, PathBuf> {
    [
        "index.html",
        "lightbox.html",
        "book/book.css",
        "toolchain/browser/module/runtime_bg.wasm",
    ]
    .into_iter()
    .map(|name| (name.to_owned(), PathBuf::from("/runfile").join(name)))
    .collect()
}

fn get(target: &str, host: &str) -> String {
    format!("GET {target} HTTP/1.1\r\nHost: {host}\r\nAccept: */*\r\n\r\n")
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
            &get(
                "/toolchain/browser/module/runtime_bg.wasm",
                "127.0.0.1:8080"
            ),
            &book,
            REPOSITORY
        ),
        Route::File {
            path: &book["toolchain/browser/module/runtime_bg.wasm"],
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
