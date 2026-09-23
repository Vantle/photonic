use std::io::{Error, ErrorKind};
use std::process::{Command, ExitCode};

fn main() -> Result<ExitCode, Error> {
    let mut argument = std::env::args_os().skip(1);
    let mut required = || {
        argument
            .next()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "missing launcher configuration"))
    };
    let executable = required()?;
    let configuration = required()?;
    let embedded = serde_json::from_slice::<Vec<String>>(&std::fs::read(configuration)?)?;
    let status = Command::new(executable)
        .args(embedded)
        .args(argument)
        .status()?;
    Ok(relay::code(status))
}
