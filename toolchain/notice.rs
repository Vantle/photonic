use relay::Failure;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: notice --output FILE (--crate 'NAME VERSION' [--manifest FILE] [--license FILE]...)... [--runtime FILE]...";

const PREAMBLE: &str =
    "Photonic is licensed under MIT OR Apache-2.0; see LICENSE-MIT and LICENSE-APACHE.

This build also contains the Rust standard library, licensed under MIT OR Apache-2.0
(https://github.com/rust-lang/rust), and the third-party crates below. Each crate is listed with
the license it declares and the license files it ships, and its source is published at
https://crates.io/crates/NAME/VERSION. A text that appears earlier is named instead of repeated.
";

struct Package {
    name: String,
    manifest: Option<PathBuf>,
    license: Vec<PathBuf>,
}

struct Request {
    output: PathBuf,
    package: Vec<Package>,
    runtime: Vec<PathBuf>,
}

fn read() -> Result<Request, Failure> {
    let mut output = None;
    let mut package: Vec<Package> = Vec::new();
    let mut runtime = Vec::new();
    let mut argument = std::env::args_os().skip(1);
    while let Some(flag) = argument.next() {
        let value = argument.next().ok_or(Failure::Usage(USAGE))?;
        match flag.to_str() {
            Some("--output") => output = Some(PathBuf::from(value)),
            Some("--crate") => package.push(Package {
                name: value.to_string_lossy().into_owned(),
                manifest: None,
                license: Vec::new(),
            }),
            Some("--manifest") => {
                package.last_mut().ok_or(Failure::Usage(USAGE))?.manifest = Some(value.into());
            }
            Some("--license") => package
                .last_mut()
                .ok_or(Failure::Usage(USAGE))?
                .license
                .push(value.into()),
            Some("--runtime") => runtime.push(value.into()),
            _ => return Err(Failure::Usage(USAGE)),
        }
    }
    Ok(Request {
        output: output.ok_or(Failure::Usage(USAGE))?,
        package,
        runtime,
    })
}

// A manifest's license field is a plain string in its package section, so a scan of that
// section's lines finds it without a TOML parser.
fn expression(manifest: &Path) -> Result<Option<String>, Failure> {
    let text = std::fs::read_to_string(manifest).map_err(Failure::access(manifest))?;
    let mut section = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            section = line == "[package]";
            continue;
        }
        let value = line
            .strip_prefix("license")
            .map(str::trim_start)
            .and_then(|rest| rest.strip_prefix('='));
        if let (true, Some(value)) = (section, value) {
            return Ok(Some(value.trim().trim_matches('"').to_owned()));
        }
    }
    Ok(None)
}

// The file's path inside its crate, without the repository that Bazel fetched it into.
fn label(path: &Path) -> String {
    let text = path.to_string_lossy();
    text.split_once("crate__")
        .and_then(|(_, rest)| rest.split_once('/'))
        .map_or_else(|| text.to_string(), |(_, file)| file.to_owned())
}

fn render(request: &Request) -> Result<String, Failure> {
    let mut text = PREAMBLE.to_owned();
    let mut seen = BTreeMap::new();
    for package in &request.package {
        let license = match &package.manifest {
            Some(manifest) => expression(manifest)?,
            None => None,
        };
        text.push_str(&format!(
            "\n{}\n{}\nLicense: {}\n",
            "-".repeat(80),
            package.name,
            license.as_deref().unwrap_or("see the files below"),
        ));
        for path in &package.license {
            let body = std::fs::read_to_string(path).map_err(Failure::access(path))?;
            let file = label(path);
            text.push_str(&format!("\n{file}\n\n"));
            let key = body.trim().to_owned();
            if let Some(first) = seen.get(&key) {
                text.push_str(&format!("The same text as {first}.\n"));
                continue;
            }
            text.push_str(body.trim_end());
            text.push('\n');
            seen.insert(key, format!("{} {file}", package.name));
        }
    }
    for path in &request.runtime {
        let body = std::fs::read_to_string(path).map_err(Failure::access(path))?;
        text.push_str(&format!("\n{}\n{}\n", "-".repeat(80), body.trim_end()));
    }
    Ok(text)
}

fn notice() -> Result<ExitCode, Failure> {
    let request = read()?;
    let text = render(&request)?;
    std::fs::write(&request.output, text).map_err(Failure::access(&request.output))?;
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    notice().unwrap_or_else(Failure::report)
}
