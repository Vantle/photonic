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

#[test]
fn launcher() -> Result<(), Box<dyn std::error::Error>> {
    let runfile = runfiles::Runfiles::create()?;
    let sample = std::env::var("SAMPLE")?;
    let executable = runfiles::rlocation!(runfile, &sample).unwrap();
    let directory = PathBuf::from(std::env::var("TEST_TMPDIR")?).join("launcher space");
    fs::create_dir_all(&directory)?;
    let manifest = directory.join("MANIFEST");
    let name = sample.strip_suffix(".exe").unwrap_or(&sample);
    let content = [
        std::env::var("PROBE")?,
        std::env::var("ARGUMENT")?,
        format!("{name}.json"),
    ]
    .map(|name| {
        format!(
            "{} {}\n",
            name,
            runfiles::rlocation!(runfile, &name).unwrap().display()
        )
    })
    .concat();
    fs::write(&manifest, content)?;
    for isolated in [false, true] {
        let mut command = Command::new(&executable);
        command.current_dir(&directory).arg("tail");
        if isolated {
            command
                .env("RUNFILES_MANIFEST_FILE", &manifest)
                .env_remove("RUNFILES_DIR")
                .env_remove("TEST_SRCDIR");
        }
        let output = command.output()?;
        assert_eq!(output.status.code(), Some(7), "{output:?}");
    }
    Ok(())
}

#[test]
fn validation() -> Result<(), Box<dyn std::error::Error>> {
    let runfile = runfiles::Runfiles::create()?;
    let locate = |name| runfiles::rlocation!(runfile, std::env::var(name).unwrap()).unwrap();
    let verify = locate("VERIFY");
    let node = locate("PHOTONIC_NODE");
    let script = locate("PHOTONIC_SCRIPT");
    let directory = PathBuf::from(std::env::var("TEST_TMPDIR")?).join("validation space");
    fs::create_dir_all(&directory)?;
    for code in [7, 0] {
        let marker = directory.join(code.to_string());
        let output = Command::new(&verify)
            .arg(&marker)
            .arg(&node)
            .arg(&script)
            .arg(code.to_string())
            .output()?;
        assert_eq!(output.status.code(), Some(code), "{output:?}");
        assert_eq!(marker.exists(), code == 0);
    }
    let marker = directory.join("missing");
    let output = Command::new(verify).arg(&marker).output()?;
    assert!(!output.status.success());
    assert!(!marker.exists());
    Ok(())
}
