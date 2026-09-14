use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn manifest() -> Result<(), Box<dyn std::error::Error>> {
    let runfile = runfiles::Runfiles::create()?;
    let locate = |name| runfiles::rlocation!(runfile, std::env::var(name).unwrap()).unwrap();
    let execute = locate("PHOTONIC_EXECUTE");
    let node = locate("PHOTONIC_NODE");
    let script = locate("PHOTONIC_SCRIPT");
    let directory = PathBuf::from(std::env::var("TEST_TMPDIR")?).join("nested runner");
    let data = directory.join("parent.exe.runfiles");
    fs::create_dir_all(&data)?;
    let parent = directory.join("parent.exe");
    fs::copy(&execute, &parent)?;
    fs::write(
        data.join("MANIFEST"),
        format!(
            "child {}\nnode {}\nscript {}\n",
            execute.display(),
            node.display(),
            script.display(),
        ),
    )?;
    for code in [0, 7] {
        let output = Command::new(&parent)
            .args(["child", "--", "node", "script", "--"])
            .arg(code.to_string())
            .env_remove("RUNFILES_DIR")
            .env_remove("RUNFILES_MANIFEST_FILE")
            .env_remove("TEST_SRCDIR")
            .output()?;
        assert_eq!(output.status.code(), Some(code), "{:?}", output);
    }
    Ok(())
}
