use relay::Failure;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn launch() -> Result<ExitCode, Failure> {
    let mut argument = std::env::args_os().skip(1);
    let mut required = || {
        argument
            .next()
            .map(PathBuf::from)
            .ok_or(Failure::Usage("missing launcher configuration"))
    };
    let executable = required()?;
    let configuration = required()?;
    let text = std::fs::read(&configuration).map_err(Failure::access(&configuration))?;
    let embedded = serde_json::from_slice::<Vec<String>>(&text)
        .map_err(std::io::Error::from)
        .map_err(Failure::access(&configuration))?;
    let status = Command::new(&executable)
        .args(embedded)
        .args(argument)
        .status()
        .map_err(Failure::access(&executable))?;
    Ok(relay::code(status))
}

fn main() -> ExitCode {
    launch().unwrap_or_else(Failure::report)
}
