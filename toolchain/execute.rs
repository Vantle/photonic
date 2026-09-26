use relay::Failure;
use std::process::{Command, ExitCode};

fn execute() -> Result<ExitCode, Failure> {
    let runfile =
        runfiles::Runfiles::create().map_err(|error| Failure::Runfile(error.to_string()))?;
    let resolve = |name: String| runfiles::rlocation!(runfile, &name).ok_or(Failure::Runfile(name));
    let mut argument = std::env::args().skip(1);
    let program = resolve(
        argument
            .next()
            .ok_or(Failure::Usage("an executable runfile is required"))?,
    )?;
    let file = argument
        .by_ref()
        .take_while(|name| name != "--")
        .map(resolve)
        .collect::<Result<Vec<_>, _>>()?;
    let mut command = Command::new(&program);
    if std::env::var_os("RUNFILES_MANIFEST_FILE").is_none_or(|value| value.is_empty()) {
        command.env(
            "RUNFILES_DIR",
            runfiles::find_runfiles_dir().map_err(|error| Failure::Runfile(error.to_string()))?,
        );
    }
    let status = command
        .args(file)
        .args(argument)
        .status()
        .map_err(Failure::access(&program))?;
    Ok(relay::code(status))
}

fn main() -> ExitCode {
    execute().unwrap_or_else(Failure::report)
}
