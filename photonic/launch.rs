use std::io::{Error, ErrorKind};
use std::process::{Command, ExitCode};

fn main() -> Result<ExitCode, Error> {
    let mut argument = std::env::args_os().skip(1);
    let executable = argument
        .next()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "missing executable"))?;
    let source = argument
        .next()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "missing source"))?;
    let runfile = runfiles::Runfiles::create().map_err(Error::other)?;
    let resolve = |name: &str| {
        runfile
            .rlocation_from(name, "")
            .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("missing runfile: {name}")))
    };
    let argument = argument.collect::<Vec<_>>();
    let operation = argument.first().and_then(|value| value.to_str());
    let explicit = matches!(operation, Some("run" | "prism" | "lower"));
    let mut command = Command::new(resolve(&executable.to_string_lossy())?);
    command
        .arg(if explicit { operation.unwrap() } else { "run" })
        .arg(resolve(&source.to_string_lossy())?);
    command.args(&argument[usize::from(explicit)..]);
    if std::env::var_os("RUNFILES_MANIFEST_FILE").is_none_or(|value| value.is_empty()) {
        command.env(
            "RUNFILES_DIR",
            runfiles::find_runfiles_dir().map_err(Error::other)?,
        );
    }
    let status = command.status()?;
    Ok(ExitCode::from(
        status
            .code()
            .and_then(|value| u8::try_from(value).ok())
            .unwrap_or(1),
    ))
}
