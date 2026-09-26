mod route;

use std::collections::HashMap;
use std::net::TcpListener;
use std::path::PathBuf;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let book = catalog()?;
    let listener = TcpListener::bind(route::ADDRESS)?;
    println!("Photonic webbook: http://{}", route::ADDRESS);
    route::serve(listener, book, env!("REPOSITORY"));
    Ok(())
}
