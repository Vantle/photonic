use relay::Failure;
use std::ffi::OsString;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const NAME: &str = "photonic";
const USAGE: &str = "usage: bazel run //:install [-- DIRECTORY]";

fn search() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default()
}

fn directory(explicit: Option<OsString>, search: &[PathBuf]) -> Result<PathBuf, Failure> {
    if let Some(explicit) = explicit {
        let base = match std::env::var_os("BUILD_WORKING_DIRECTORY") {
            Some(base) => PathBuf::from(base),
            None => std::env::current_dir().map_err(Failure::access(Path::new(".")))?,
        };
        return Ok(base.join(explicit));
    }
    let home = std::env::home_dir()
        .filter(|home| home.is_absolute())
        .ok_or(Failure::Usage(
            "no home directory; name one with bazel run //:install -- DIRECTORY",
        ))?;
    let conventional = [
        home.join(".local").join("bin"),
        home.join("bin"),
        home.join(".bin"),
    ];
    let found = conventional
        .iter()
        .find(|candidate| search.contains(candidate));
    Ok(found.unwrap_or(&conventional[0]).clone())
}

fn clear(path: &Path) -> Result<(), Failure> {
    let result = std::fs::remove_file(path);
    if result
        .as_ref()
        .is_err_and(|error| error.kind() == ErrorKind::NotFound)
    {
        return Ok(());
    }
    result.map_err(Failure::access(path))
}

#[cfg(unix)]
fn permit(path: &Path) -> Result<(), Failure> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .map_err(Failure::access(path))
}

#[cfg(not(unix))]
fn permit(_path: &Path) -> Result<(), Failure> {
    Ok(())
}

fn check(path: &Path) -> Result<String, Failure> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(Failure::access(path))?;
    if !output.status.success() {
        return Err(Failure::Tool {
            path: path.to_path_buf(),
            message: format!("does not run ({})", output.status),
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn place(source: &Path, directory: &Path) -> Result<(PathBuf, String), Failure> {
    std::fs::create_dir_all(directory).map_err(Failure::access(directory))?;
    let suffix = std::env::consts::EXE_SUFFIX;
    let target = directory.join(format!("{NAME}{suffix}"));
    let partial = directory.join(format!(".{NAME}.partial{suffix}"));
    clear(&partial)?;
    std::fs::copy(source, &partial).map_err(Failure::access(&partial))?;
    permit(&partial)?;
    let version = check(&partial).inspect_err(|_| {
        let _ = std::fs::remove_file(&partial);
    })?;
    std::fs::rename(&partial, &target).map_err(Failure::access(&target))?;
    Ok((target, version))
}

fn install() -> Result<ExitCode, Failure> {
    let mut argument = std::env::args_os().skip(1);
    let name = argument
        .next()
        .ok_or(Failure::Usage("the photonic runfile is required"))?;
    let explicit = argument.next();
    if argument.next().is_some()
        || explicit
            .as_ref()
            .is_some_and(|value| value.as_encoded_bytes().starts_with(b"-"))
    {
        return Err(Failure::Usage(USAGE));
    }
    let name = name.to_string_lossy().into_owned();
    let runfile =
        runfiles::Runfiles::create().map_err(|error| Failure::Runfile(error.to_string()))?;
    let source = runfiles::rlocation!(runfile, &name).ok_or(Failure::Runfile(name))?;
    let search = search();
    let directory = directory(explicit, &search)?;
    let (target, version) = place(&source, &directory)?;
    println!("installed {version} at {}", target.display());
    if search.contains(&directory) {
        return Ok(ExitCode::SUCCESS);
    }
    println!(
        "{} is not on your PATH; add it to run {NAME} by name",
        directory.display()
    );
    if cfg!(unix) {
        println!("  export PATH=\"{}:$PATH\"", directory.display());
    }
    Ok(ExitCode::SUCCESS)
}

fn main() -> ExitCode {
    install().unwrap_or_else(Failure::report)
}
