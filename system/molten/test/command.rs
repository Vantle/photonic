use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let base = std::env::var_os("TEST_TMPDIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = base.join(format!(
            "molten-{}-{time}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self { path }
    }

    fn write(&self, name: &str, source: &str) -> PathBuf {
        let path = self.path.join(name);
        std::fs::write(&path, source).unwrap();
        path
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn execute(operation: &str, path: &Path, argument: &[&str]) -> Output {
    let binary = std::env::var_os("MOLTEN_COMMAND")
        .map(PathBuf::from)
        .or_else(|| option_env!("CARGO_BIN_EXE_molten").map(PathBuf::from))
        .expect("MOLTEN_COMMAND or Cargo binary path");
    Command::new(binary)
        .arg(operation)
        .arg(path)
        .args(argument)
        .output()
        .expect("execute Molten")
}

fn report(output: &Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON execution report")
}

#[test]
fn execution() {
    let fixture = Fixture::new();
    let path = fixture.write("program.lava", "Seed.A [Seed] [A] B");
    let output = execute("run", &path, &["--json"]);
    let result = report(&output);
    assert_eq!(result["closed"], true);
    assert!(result["state"].as_array().unwrap().iter().any(|state| {
        if state["status"] != "supported" {
            return false;
        }
        let world = state["world"].as_array().unwrap();
        if world.len() != 1 {
            return false;
        }
        let mut particle = world[0]["particle"]
            .as_array()
            .unwrap()
            .iter()
            .map(|token| token["display"].as_str().unwrap())
            .collect::<Vec<_>>();
        particle.sort();
        particle == ["B", "Seed"]
    }));
    let output = execute("run", &path, &[]);
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.starts_with("Closed:"));
    assert!(text.contains("supported"));
    assert!(text.contains("@root"));
    let output = execute(
        "run",
        &path,
        &[
            "--json",
            "--steps",
            "2",
            "--states",
            "1",
            "--cells",
            "5",
            "--frames",
            "3",
            "--coherences",
            "2",
        ],
    );
    let result = report(&output);
    assert_eq!(result["closed"], false);
    assert_eq!(result["limit"]["state"], 1);
    assert_eq!(result["limit"]["cell"], 5);
    assert_eq!(result["limit"]["frame"], 3);
    assert_eq!(result["limit"]["world"], 2);
    assert!(result["work"].as_u64().unwrap() <= 2);
}

#[test]
fn format() {
    let fixture = Fixture::new();
    let source = r#"{"initial":[["A"]],"rule":[{"input":[["A"]],"output":[{"particle":["B"]}]}]}"#;
    let path = fixture.write("program.json", source);
    let inferred = report(&execute("run", &path, &["--json"]));
    let path = fixture.write("program.lava", source);
    let explicit = report(&execute("run", &path, &["--format", "json", "--json"]));
    assert_eq!(inferred, explicit);
    assert_eq!(inferred["closed"], true);
    let path = fixture.write("native.json", "A [A] B");
    let native = report(&execute("run", &path, &["--format", "molten", "--json"]));
    assert_eq!(native["closed"], true);
    let path = fixture.write("broken.json", "{");
    let output = execute("run", &path, &[]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid JSON program"));
}

#[test]
fn diagnostic() {
    let fixture = Fixture::new();
    let path = fixture.write("invalid.lava", "人]");
    let output = execute("run", &path, &[]);
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("molten::lowering"));
    assert!(error.contains("invalid.lava"));
    assert!(error.contains("人]"));
    let path = fixture.write("structure.lava", "人.世界 [人]");
    let output = execute("parse", &path, &[]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Concept"));
    let output = execute("run", &fixture.path.join("missing.lava"), &[]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("could not read"));
}

#[test]
fn worker() {
    let fixture = Fixture::new();
    let path = fixture.write("program.lava", "Seed.A [Seed] [A] B");
    let sequential = report(&execute("run", &path, &["--json"]));
    let parallel = report(&execute("run", &path, &["--json", "--workers", "4"]));
    assert_eq!(sequential, parallel);
    let paused = report(&execute("run", &path, &["--json", "--records", "1"]));
    assert_eq!(paused["closed"], false);
    assert_eq!(paused["work"], 0);
    let invalid = execute("run", &path, &["--workers", "0"]);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("at least one worker"));
}

#[test]
fn obsidian() {
    let fixture = Fixture::new();
    let path = fixture.write("program.lava", "A [A] B");
    let target = fixture.write("target.lava", "B");
    let result = report(&execute(
        "obsidian",
        &path,
        &["--target", target.to_str().unwrap(), "--json"],
    ));
    assert_eq!(result["outcome"], "reached");
    assert!(result["witness"].is_u64());
    assert_eq!(result["program"]["initial"][0][0], "A");
    assert_eq!(result["target"][0][0], "B");
    let result = report(&execute(
        "obsidian",
        &path,
        &[
            "--target",
            target.to_str().unwrap(),
            "--json",
            "--steps",
            "0",
        ],
    ));
    assert_eq!(result["outcome"], "unknown");
    let invalid = fixture.write("invalid.lava", "B [B] A");
    assert!(
        !execute("obsidian", &path, &["--target", invalid.to_str().unwrap()])
            .status
            .success()
    );
    let output = execute("obsidian", &path, &["--target", target.to_str().unwrap()]);
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .starts_with("Reached:")
    );
}

#[test]
fn depth() {
    let fixture = Fixture::new();
    let path = fixture.write("deep.lava", &"[A] ".repeat(10_000));
    let output = execute("run", &path, &["--steps", "0"]);
    assert!(!output.status.success());
    assert!(output.status.code().is_some());
    assert!(String::from_utf8_lossy(&output.stderr).contains("molten::depth"));
}
