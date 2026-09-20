use std::io::{Error, ErrorKind};
use std::process::{Command, ExitCode};

fn main() -> Result<ExitCode, Error> {
    let mut argument = std::env::args_os().skip(1);
    let output = argument
        .next()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "a success marker is required"))?;
    let executable = argument
        .next()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "an executable is required"))?;
    let status = Command::new(executable).args(argument).status()?;
    if !status.success() {
        return Ok(ExitCode::from(
            status
                .code()
                .and_then(|code| code.try_into().ok())
                .unwrap_or(1),
        ));
    }
    std::fs::write(output, [])?;
    Ok(ExitCode::SUCCESS)
}
