use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn runfile(variable: &str) -> PathBuf {
    let name = std::env::var(variable).expect("a runfile variable");
    let runfile = runfiles::Runfiles::create().expect("runfiles");
    runfiles::rlocation!(runfile, &name).expect("a runfile")
}

fn scratch(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os("TEST_TMPDIR").expect("a test directory")).join(name)
}

fn executable(directory: &Path) -> PathBuf {
    directory.join(format!("photonic{}", std::env::consts::EXE_SUFFIX))
}

fn partial(directory: &Path) -> PathBuf {
    directory.join(format!(".photonic.partial{}", std::env::consts::EXE_SUFFIX))
}

fn install(source: &str, home: &Path, search: &[PathBuf], argument: &[&OsStr]) -> Output {
    Command::new(runfile("INSTALL"))
        .arg(std::env::var(source).expect("an installable runfile"))
        .args(argument)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("PATH", std::env::join_paths(search).expect("a search path"))
        .env("BUILD_WORKING_DIRECTORY", home)
        .output()
        .expect("the installer runs")
}

fn success(output: &Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn failure(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn explicit() {
    let home = scratch("explicit");
    let directory = home.join("tool");
    let search = [directory.clone()];
    let first = success(&install(
        "RELEASE",
        &home,
        &search,
        &[directory.as_os_str()],
    ));
    let again = success(&install("RELEASE", &home, &search, &[OsStr::new("tool")]));
    let target = executable(&directory);
    assert!(first.contains("installed photonic"), "{first}");
    assert!(first.contains(&target.display().to_string()), "{first}");
    assert!(!first.contains("not on your PATH"), "{first}");
    assert_eq!(first, again);
    let help = Command::new(&target)
        .arg("--help")
        .output()
        .expect("the installed command runs");
    let text = String::from_utf8_lossy(&help.stdout);
    for tool in ["prism", "check", "explore", "cause", "mcp"] {
        assert!(text.contains(tool), "{text}");
    }
    assert!(!partial(&directory).exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&target)
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
    }
}

#[test]
fn automatic() {
    let elsewhere = scratch("elsewhere");
    for (name, search, chosen) in [
        ("unlisted", vec![], ".local/bin"),
        ("local", vec![".local/bin"], ".local/bin"),
        ("plain", vec![".bin", "bin"], "bin"),
        ("hidden", vec![".bin"], ".bin"),
    ] {
        let home = scratch(name);
        let search = search
            .into_iter()
            .map(|directory| home.join(directory))
            .chain([elsewhere.clone()])
            .collect::<Vec<_>>();
        let output = success(&install("RELEASE", &home, &search, &[]));
        let directory = home.join(chosen).components().collect::<PathBuf>();
        let target = executable(&directory);
        assert!(target.is_file(), "{name}: {output}");
        assert!(
            output.contains(&format!("at {}", target.display())),
            "{name}: {output}"
        );
        assert_eq!(
            output.contains("not on your PATH"),
            !search.contains(&directory),
            "{name}: {output}"
        );
    }
}

#[test]
fn argument() {
    let home = scratch("argument");
    for argument in [&["--help"][..], &["-"], &["first", "second"]] {
        let argument = argument.iter().map(OsStr::new).collect::<Vec<_>>();
        let output = failure(&install("RELEASE", &home, &[], &argument));
        assert!(
            output.contains("usage: bazel run //:install [-- DIRECTORY]"),
            "{output}"
        );
    }
    assert!(!home.exists());
}

#[test]
fn broken() {
    let home = scratch("broken");
    let directory = home.join("bin");
    let search = [directory.clone()];
    success(&install("RELEASE", &home, &search, &[]));
    let output = failure(&install("PROBE", &home, &search, &[]));
    assert!(output.contains("does not run"), "{output}");
    assert!(!partial(&directory).exists());
    let version = Command::new(executable(&directory))
        .arg("--version")
        .output()
        .expect("the installed command runs");
    assert!(version.status.success());
    assert!(String::from_utf8_lossy(&version.stdout).contains("photonic"));
}
