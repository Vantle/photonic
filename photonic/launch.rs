use std::io::{Error, ErrorKind};
use std::process::{Command, ExitCode};

// The command travels in the launcher's own runfiles, so a binary built in another module names
// only its program and never depends on the command's target.
const COMMAND: &str = env!("COMMAND");

fn main() -> Result<ExitCode, Error> {
    let mut argument = std::env::args_os().skip(1);
    let program = argument
        .next()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "missing program"))?;
    let runfile = runfiles::Runfiles::create().map_err(Error::other)?;
    let resolve = |name: &str| {
        runfile
            .rlocation_from(name, "")
            .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("missing runfile: {name}")))
    };
    let argument = argument.collect::<Vec<_>>();
    let verb = argument
        .first()
        .and_then(|value| value.to_str())
        .filter(|value| !value.starts_with('-'));
    let mut command = Command::new(resolve(COMMAND)?);
    command
        .arg(verb.unwrap_or("run"))
        .arg(resolve(&program.to_string_lossy())?);
    command.args(&argument[usize::from(verb.is_some())..]);
    Ok(relay::code(command.status()?))
}
