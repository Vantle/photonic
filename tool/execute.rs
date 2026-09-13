use std::io::{Error, ErrorKind};
use std::process::{Command, ExitCode};

fn main() -> Result<ExitCode, Error> {
    let runfile = runfiles::Runfiles::create().map_err(Error::other)?;
    let mut argument = std::env::args().skip(1).map(|name| {
        runfiles::rlocation!(runfile, &name)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("missing runfile: {name}")))
    });
    let program = argument.next().ok_or_else(|| {
        Error::new(ErrorKind::InvalidInput, "an executable runfile is required")
    })??;
    let status = Command::new(program)
        .args(argument.collect::<Result<Vec<_>, _>>()?)
        .status()?;
    Ok(ExitCode::from(
        status
            .code()
            .and_then(|code| code.try_into().ok())
            .unwrap_or(1),
    ))
}
