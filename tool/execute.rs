use std::io::{Error, ErrorKind};
use std::process::{Command, ExitCode};

fn main() -> Result<ExitCode, Error> {
    let runfile = runfiles::Runfiles::create().map_err(Error::other)?;
    let resolve = |name: String| {
        runfiles::rlocation!(runfile, &name)
            .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("missing runfile: {name}")))
    };
    let mut argument = std::env::args().skip(1);
    let program = resolve(argument.next().ok_or_else(|| {
        Error::new(ErrorKind::InvalidInput, "an executable runfile is required")
    })?)?;
    let file = argument
        .by_ref()
        .take_while(|name| name != "--")
        .map(resolve)
        .collect::<Result<Vec<_>, _>>()?;
    let status = Command::new(program).args(file).args(argument).status()?;
    Ok(ExitCode::from(
        status
            .code()
            .and_then(|code| code.try_into().ok())
            .unwrap_or(1),
    ))
}
