use std::ffi::OsString;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const NAME: &str = "photonic";

fn search() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default()
}

fn directory(explicit: Option<OsString>, search: &[PathBuf]) -> Result<PathBuf, Error> {
    if let Some(explicit) = explicit {
        let base = std::env::var_os("BUILD_WORKING_DIRECTORY")
            .map_or_else(std::env::current_dir, |base| Ok(PathBuf::from(base)))?;
        return Ok(base.join(explicit));
    }
    let home = std::env::home_dir()
        .filter(|home| home.is_absolute())
        .ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                "no home directory; name one with bazel run //:install -- DIRECTORY",
            )
        })?;
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

fn clear(path: &Path) -> Result<(), Error> {
    match std::fs::remove_file(path) {
        Err(error) if error.kind() != ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

#[cfg(unix)]
fn permit(path: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
}

#[cfg(not(unix))]
fn permit(_path: &Path) -> Result<(), Error> {
    Ok(())
}

fn place(source: &Path, directory: &Path) -> Result<PathBuf, Error> {
    std::fs::create_dir_all(directory)?;
    let name = format!("{NAME}{}", std::env::consts::EXE_SUFFIX);
    let target = directory.join(&name);
    let partial = directory.join(format!(".{name}.partial"));
    clear(&partial)?;
    std::fs::copy(source, &partial)?;
    permit(&partial)?;
    std::fs::rename(&partial, &target)?;
    Ok(target)
}

fn main() -> Result<ExitCode, Error> {
    let mut argument = std::env::args_os().skip(1);
    let name = argument
        .next()
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "the photonic runfile is required"))?;
    let name = name.to_string_lossy().into_owned();
    let runfile = runfiles::Runfiles::create().map_err(Error::other)?;
    let source = runfiles::rlocation!(runfile, &name)
        .ok_or_else(|| Error::new(ErrorKind::NotFound, format!("missing runfile: {name}")))?;
    let search = search();
    let directory = directory(argument.next(), &search)?;
    let target = place(&source, &directory)?;
    let version = Command::new(&target).arg("--version").output()?;
    if !version.status.success() {
        return Err(Error::other(format!(
            "{} was installed but does not run",
            target.display()
        )));
    }
    println!(
        "installed {} at {}",
        String::from_utf8_lossy(&version.stdout).trim(),
        target.display()
    );
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
