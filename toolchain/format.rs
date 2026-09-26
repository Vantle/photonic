use relay::Failure;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

fn source(label: &str) -> Option<PathBuf> {
    let (package, file) = label.strip_prefix("//")?.split_once(':')?;
    Some(PathBuf::from(package).join(file))
}

fn query(workspace: &Path, expression: &str) -> Result<Vec<PathBuf>, Failure> {
    let bazel = PathBuf::from(std::env::var_os("BAZEL_REAL").unwrap_or_else(|| "bazel".into()));
    let output = Command::new(&bazel)
        .current_dir(workspace)
        .args(["query", expression, "--output=label"])
        .stderr(Stdio::inherit())
        .output()
        .map_err(Failure::access(&bazel))?;
    let failure = |message: String| Failure::Tool {
        path: bazel.clone(),
        message,
    };
    if !output.status.success() {
        return Err(failure("source discovery failed".to_owned()));
    }
    let text = String::from_utf8(output.stdout).map_err(|error| failure(error.to_string()))?;
    let mut file = text
        .lines()
        .map(|label| source(label).ok_or_else(|| failure(format!("invalid source label: {label}"))))
        .collect::<Result<Vec<_>, _>>()?;
    file.sort();
    file.dedup();
    if file.is_empty() {
        return Err(failure(format!("no sources match {expression}")));
    }
    Ok(file)
}

fn tool(runfile: &runfiles::Runfiles, name: &str) -> Result<PathBuf, Failure> {
    runfiles::rlocation!(runfile, name).ok_or_else(|| Failure::Runfile(name.to_owned()))
}

fn format() -> Result<ExitCode, Failure> {
    let workspace = PathBuf::from(
        std::env::var_os("BUILD_WORKSPACE_DIRECTORY")
            .ok_or(Failure::Usage("run the formatter with bazel run //:format"))?,
    );
    let argument = std::env::args().skip(1).collect::<Vec<_>>();
    let [rustfmt, buildifier, option @ ..] = argument.as_slice() else {
        return Err(Failure::Usage(
            "the rustfmt and buildifier runfiles are required",
        ));
    };
    let check = option.iter().any(|option| option == "--check");
    let runfile =
        runfiles::Runfiles::create().map_err(|error| Failure::Runfile(error.to_string()))?;
    let rust = query(
        &workspace,
        "filter('^//.*[.]rs$', kind('source file', deps(kind('rust_.* rule', //...))))",
    )?;
    let starlark = query(&workspace, "kind('source file', deps(//:build))")?;
    let rustfmt = tool(&runfile, rustfmt)?;
    let status = Command::new(&rustfmt)
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
        .status()
        .map_err(Failure::access(&rustfmt))?;
    if !status.success() {
        return Ok(relay::code(status));
    }
    let mode = if check {
        ["-mode=check", "-lint=warn"].as_slice()
    } else {
        ["-mode=fix"].as_slice()
    };
    let buildifier = tool(&runfile, buildifier)?;
    let status = Command::new(&buildifier)
        .current_dir(&workspace)
        .args(mode)
        .args(starlark)
        .status()
        .map_err(Failure::access(&buildifier))?;
    Ok(relay::code(status))
}

fn main() -> ExitCode {
    format().unwrap_or_else(Failure::report)
}
