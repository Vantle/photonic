use std::io::{Error, ErrorKind};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};

fn source(label: &str) -> Result<PathBuf, Error> {
    let (package, file) = label
        .strip_prefix("//")
        .and_then(|label| label.split_once(':'))
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidData,
                format!("invalid source label: {label}"),
            )
        })?;
    Ok(PathBuf::from(package).join(file))
}

fn main() -> Result<ExitCode, Error> {
    let workspace = std::env::var_os("BUILD_WORKSPACE_DIRECTORY").ok_or_else(|| {
        Error::new(
            ErrorKind::NotFound,
            "run the formatter with bazel run //:format",
        )
    })?;
    let mut argument = std::env::args().skip(1);
    let name = argument
        .next()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "a formatter runfile is required"))?;
    let runfile = runfiles::Runfiles::create().map_err(Error::other)?;
    let executable = runfiles::rlocation!(runfile, &name)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("missing runfile: {name}")))?;
    let query = Command::new(std::env::var_os("BAZEL_REAL").unwrap_or_else(|| "bazel".into()))
        .current_dir(&workspace)
        .args([
            "query",
            "filter('^//.*[.]rs$', kind('source file', deps(kind('rust_.* rule', //...))))",
            "--output=label",
        ])
        .stderr(Stdio::inherit())
        .output()?;
    if !query.status.success() {
        return Err(Error::other("Bazel source discovery failed"));
    }
    let label = String::from_utf8(query.stdout).map_err(Error::other)?;
    let mut file = label.lines().map(source).collect::<Result<Vec<_>, _>>()?;
    file.sort();
    file.dedup();
    if file.is_empty() {
        return Err(Error::new(
            ErrorKind::NotFound,
            "no Rust source targets found",
        ));
    }
    let status = Command::new(executable)
        .current_dir(workspace)
        .args([
            "--config-path",
            "rustfmt.toml",
            "--config",
            "skip_children=true",
        ])
        .args(argument)
        .args(file)
        .status()?;
    Ok(ExitCode::from(
        status
            .code()
            .and_then(|code| code.try_into().ok())
            .unwrap_or(1),
    ))
}
