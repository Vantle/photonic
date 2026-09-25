use std::io::{Error, ErrorKind};
use std::process::{Command, ExitCode};

fn main() -> Result<ExitCode, Error> {
    let workspace = std::env::var_os("BUILD_WORKSPACE_DIRECTORY")
        .ok_or_else(|| Error::new(ErrorKind::NotFound, "run this tool with bazel run"))?;
    let mut argument = std::env::args().skip(1);
    let name = argument
        .next()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "a tool runfile is required"))?;
    let runfile = runfiles::Runfiles::create().map_err(Error::other)?;
    let tool = runfiles::rlocation!(runfile, &name)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("missing runfile: {name}")))?;
    let status = Command::new(tool)
        .current_dir(&workspace)
        .env("BUILD_WORKING_DIRECTORY", &workspace)
        .args(argument)
        .status()?;
    Ok(relay::code(status))
}
