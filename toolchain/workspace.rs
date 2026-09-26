use relay::Failure;
use std::process::{Command, ExitCode};

fn run() -> Result<ExitCode, Failure> {
    let workspace = std::env::var_os("BUILD_WORKSPACE_DIRECTORY")
        .ok_or(Failure::Usage("run this tool with bazel run"))?;
    let mut argument = std::env::args().skip(1);
    let name = argument
        .next()
        .ok_or(Failure::Usage("a tool runfile is required"))?;
    let runfile =
        runfiles::Runfiles::create().map_err(|error| Failure::Runfile(error.to_string()))?;
    let tool = runfiles::rlocation!(runfile, &name).ok_or(Failure::Runfile(name))?;
    let status = Command::new(&tool)
        .current_dir(&workspace)
        .env("BUILD_WORKING_DIRECTORY", &workspace)
        .args(argument)
        .status()
        .map_err(Failure::access(&tool))?;
    Ok(relay::code(status))
}

fn main() -> ExitCode {
    run().unwrap_or_else(Failure::report)
}
