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

// Windows compares paths without regard to case or a trailing separator; other systems compare
// them exactly.
fn same(left: &Path, right: &Path) -> bool {
    if !cfg!(windows) {
        return left == right;
    }
    let normal = |path: &Path| {
        path.to_string_lossy()
            .trim_end_matches(['\\', '/'])
            .to_lowercase()
    };
    normal(left) == normal(right)
}

fn listed(search: &[PathBuf], directory: &Path) -> bool {
    search.iter().any(|entry| same(entry, directory))
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
        .find(|candidate| listed(search, candidate));
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
    replace(&partial, &target, directory, suffix)?;
    Ok((target, version))
}

// Windows refuses to overwrite a running executable but lets it be renamed, so the installed
// copy moves aside first; other systems replace it in one rename.
fn replace(partial: &Path, target: &Path, directory: &Path, suffix: &str) -> Result<(), Failure> {
    if !cfg!(windows) || !target.exists() {
        return std::fs::rename(partial, target).map_err(Failure::access(target));
    }
    let previous = directory.join(format!(".{NAME}.previous{suffix}"));
    clear(&previous)?;
    std::fs::rename(target, &previous).map_err(Failure::access(target))?;
    if let Err(error) = std::fs::rename(partial, target) {
        let _ = std::fs::rename(&previous, target);
        return Err(Failure::access(target)(error));
    }
    let _ = std::fs::remove_file(&previous);
    Ok(())
}

// The first file on the search path that running the command by name would start.
fn resolve(search: &[PathBuf], suffix: &str) -> Option<PathBuf> {
    search
        .iter()
        .map(|directory| directory.join(format!("{NAME}{suffix}")))
        .find(|candidate| candidate.is_file())
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
    if !listed(&search, &directory) {
        advise(&directory);
        return Ok(ExitCode::SUCCESS);
    }
    let first = resolve(&search, std::env::consts::EXE_SUFFIX);
    if let Some(first) = first.filter(|first| !same(first, &target)) {
        println!(
            "{} comes first on your PATH, so {NAME} still runs it; remove it or move {} earlier",
            first.display(),
            directory.display()
        );
    }
    Ok(ExitCode::SUCCESS)
}

fn advise(directory: &Path) {
    println!(
        "{} is not on your PATH; add it to run {NAME} by name",
        directory.display()
    );
    if cfg!(windows) {
        println!(
            "  [Environment]::SetEnvironmentVariable(\"Path\", \"{};\" + [Environment]::GetEnvironmentVariable(\"Path\", \"User\"), \"User\")",
            directory.display()
        );
        println!("then open a new terminal");
        return;
    }
    println!("add this line to your shell's profile, such as ~/.zshrc or ~/.bashrc:");
    println!("  export PATH=\"{}:$PATH\"", directory.display());
}

fn main() -> ExitCode {
    install().unwrap_or_else(Failure::report)
}
