mod route;

use std::collections::HashMap;
use std::fmt;
use std::io::ErrorKind;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::ExitCode;

enum Failure {
    Runfile(runfiles::RunfilesError),
    Catalog,
    File(String),
    Occupied,
    Listen(std::io::Error),
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runfile(error) => write!(
                formatter,
                "The server could not find its runfiles ({error}). Start it with bazel run -c opt //book:serve."
            ),
            Self::Catalog => write!(
                formatter,
                "The server does not know which files make up the book. Start it with bazel run -c opt //book:serve."
            ),
            Self::File(location) => write!(
                formatter,
                "The book's file {location} is missing from the server's runfiles."
            ),
            Self::Occupied => write!(
                formatter,
                "Another program is already listening on {}. Stop it, or open http://{} if it is this book.",
                route::ADDRESS,
                route::ADDRESS
            ),
            Self::Listen(error) => write!(
                formatter,
                "The server could not listen on {}: {error}",
                route::ADDRESS
            ),
        }
    }
}

fn catalog() -> Result<HashMap<String, PathBuf>, Failure> {
    let runfile = runfiles::Runfiles::create().map_err(Failure::Runfile)?;
    std::env::var("BOOK")
        .map_err(|_| Failure::Catalog)?
        .split_whitespace()
        .map(|location| {
            let missing = || Failure::File(location.to_owned());
            let (_, path) = location.split_once('/').ok_or_else(missing)?;
            let file = runfile.rlocation_from(location, "").ok_or_else(missing)?;
            Ok((path.to_owned(), file))
        })
        .collect()
}

fn listen() -> Result<TcpListener, Failure> {
    TcpListener::bind(route::ADDRESS).map_err(|error| match error.kind() {
        ErrorKind::AddrInUse => Failure::Occupied,
        _ => Failure::Listen(error),
    })
}

fn main() -> ExitCode {
    let started = catalog().and_then(|book| Ok((book, listen()?)));
    let (book, listener) = match started {
        Ok(started) => started,
        Err(failure) => {
            eprintln!("{failure}");
            return ExitCode::FAILURE;
        }
    };
    println!("Photonic webbook: http://{}", route::ADDRESS);
    route::serve(listener, book, env!("REPOSITORY"));
    ExitCode::SUCCESS
}
