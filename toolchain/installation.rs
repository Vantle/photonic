use std::path::{Path, PathBuf};
use std::process::Command;

fn runfile(variable: &str) -> PathBuf {
    let name = std::env::var(variable).expect("a runfile variable");
    let runfile = runfiles::Runfiles::create().expect("runfiles");
    runfiles::rlocation!(runfile, &name).expect("a runfile")
}

fn install(directory: &Path) -> String {
    let output = Command::new(runfile("INSTALL"))
        .arg(std::env::var("RELEASE").expect("the release runfile"))
        .arg(directory)
        .env("PATH", directory)
        .output()
        .expect("the installer runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn installation() {
    let directory =
        PathBuf::from(std::env::var_os("TEST_TMPDIR").expect("a test directory")).join("bin");
    let first = install(&directory);
    let again = install(&directory);
    let target = directory.join(format!("photonic{}", std::env::consts::EXE_SUFFIX));
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
    assert!(!directory.join(".photonic.partial").exists());
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
