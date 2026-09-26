use relay::Failure;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn verify() -> Result<ExitCode, Failure> {
    let mut argument = std::env::args_os().skip(1);
    let output = argument
        .next()
        .map(PathBuf::from)
        .ok_or(Failure::Usage("a success marker is required"))?;
    let executable = argument
        .next()
        .map(PathBuf::from)
        .ok_or(Failure::Usage("an executable is required"))?;
    let status = Command::new(&executable)
        .args(argument)
        .status()
        .map_err(Failure::access(&executable))?;
    if !status.success() {
        return Ok(relay::code(status));
    }
    std::fs::write(&output, []).map_err(Failure::access(&output))?;
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    verify().unwrap_or_else(Failure::report)
}
