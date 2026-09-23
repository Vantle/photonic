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
    let mut command = Command::new(program);
    if std::env::var_os("RUNFILES_MANIFEST_FILE").is_none_or(|value| value.is_empty()) {
        command.env(
            "RUNFILES_DIR",
            runfiles::find_runfiles_dir().map_err(Error::other)?,
        );
    }
    let status = command.args(file).args(argument).status()?;
    Ok(relay::code(status))
}
