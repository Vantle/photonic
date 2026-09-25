use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
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

fn query(workspace: &Path, expression: &str) -> Result<Vec<PathBuf>, Error> {
    let output = Command::new(std::env::var_os("BAZEL_REAL").unwrap_or_else(|| "bazel".into()))
        .current_dir(workspace)
        .args(["query", expression, "--output=label"])
        .stderr(Stdio::inherit())
        .output()?;
    if !output.status.success() {
        return Err(Error::other("Bazel source discovery failed"));
    }
    let label = String::from_utf8(output.stdout).map_err(Error::other)?;
    let mut file = label.lines().map(source).collect::<Result<Vec<_>, _>>()?;
    file.sort();
    file.dedup();
    if file.is_empty() {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!("no sources match {expression}"),
        ));
    }
    Ok(file)
}

fn tool(runfile: &runfiles::Runfiles, name: &str) -> Result<PathBuf, Error> {
    runfiles::rlocation!(runfile, name)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("missing runfile: {name}")))
}

fn main() -> Result<ExitCode, Error> {
    let workspace = PathBuf::from(std::env::var_os("BUILD_WORKSPACE_DIRECTORY").ok_or_else(
        || {
            Error::new(
                ErrorKind::NotFound,
                "run the formatter with bazel run //:format",
            )
        },
    )?);
    let argument = std::env::args().skip(1).collect::<Vec<_>>();
    let [rustfmt, buildifier, option @ ..] = argument.as_slice() else {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "the rustfmt and buildifier runfiles are required",
        ));
    };
    let check = option.iter().any(|option| option == "--check");
    let runfile = runfiles::Runfiles::create().map_err(Error::other)?;
    let rust = query(
        &workspace,
        "filter('^//.*[.]rs$', kind('source file', deps(kind('rust_.* rule', //...))))",
    )?;
    let starlark = query(&workspace, "kind('source file', deps(//:build))")?;
    let status = Command::new(tool(&runfile, rustfmt)?)
        .current_dir(&workspace)
        .env("BUILD_WORKING_DIRECTORY", &workspace)
        .args([
            "--config-path",
            "rustfmt.toml",
            "--config",
            "skip_children=true",
        ])
        .args(option)
        .args(rust)
        .status()?;
    if !status.success() {
        return Ok(relay::code(status));
    }
    let mode = if check {
        ["-mode=check", "-lint=warn"].as_slice()
    } else {
        ["-mode=fix"].as_slice()
    };
    let status = Command::new(tool(&runfile, buildifier)?)
        .current_dir(&workspace)
        .args(mode)
        .args(starlark)
        .status()?;
    Ok(relay::code(status))
}
