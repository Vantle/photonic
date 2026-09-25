use learning::encoding::{DIMENSION, Shape};
use learning::home::{CHECKPOINT, POOL};
use network::checkpoint;
use network::model::Model;
use network::optimizer::{self, Optimizer};
use random::Generator;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT: AtomicUsize = AtomicUsize::new(0);

struct Fixture {
    path: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let base = std::env::var_os("TEST_TMPDIR").map_or_else(std::env::temp_dir, PathBuf::from);
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = base.join(format!(
            "learning-{}-{time}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(path.join("home")).unwrap();
        Self { path }
    }

    fn home(&self) -> String {
        self.path.join("home").display().to_string()
    }

    fn write(&self, name: &str, text: &str) -> String {
        let path = self.path.join(name);
        std::fs::write(&path, text).unwrap();
        path.display().to_string()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn execute(argument: &[&str]) -> Output {
    let binary = std::env::var_os("LEARNING_COMMAND").expect("learning runfile path");
    let binary = runfiles::Runfiles::create()
        .expect("Bazel runfiles")
        .rlocation_from(binary, "")
        .expect("learning executable");
    Command::new(binary)
        .args(argument)
        .env("NO_COLOR", "1")
        .output()
        .expect("execute learning")
}

fn flat(text: &[u8]) -> String {
    String::from_utf8_lossy(text)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn succeed(argument: &[&str]) -> String {
    let output = execute(argument);
    assert!(output.status.success(), "{}", flat(&output.stderr));
    flat(&output.stdout)
}

fn fail(argument: &[&str]) -> String {
    let output = execute(argument);
    assert!(!output.status.success(), "{argument:?}");
    flat(&output.stderr)
}

#[test]
fn objective() {
    for (argument, message) in [
        (["status", "--processor=0"], "processor count"),
        (["status", "--processor=inf"], "processor count"),
        (["status", "--size=-0.5"], "size weight"),
        (["status", "--size=NaN"], "size weight"),
        (["solve", "--processor=-2"], "processor count"),
        (["train", "--size=inf"], "size weight"),
    ] {
        assert!(fail(&argument).contains(message), "{argument:?}");
    }
}

#[test]
fn measure() {
    let fixture = Fixture::new();
    let home = fixture.home();
    assert!(fail(&["status", "--home", &home, "--processor", "2"]).contains("--task"));
    assert!(succeed(&["status", "--home", &home]).contains("pool: 0 tasks"));
    let message = fail(&[
        "status", "--home", &home, "--task", "missing", "--size", "0.1",
    ]);
    assert!(message.contains("no task is named missing"));
}

#[test]
fn name() {
    let fixture = Fixture::new();
    let input = fixture.write("input.wave", "A");
    let output = fixture.write("output.wave", "B");
    let message = fail(&[
        "solve",
        "--home",
        &fixture.home(),
        "--name",
        "",
        "--input",
        &input,
        "--output",
        &output,
    ]);
    assert!(message.contains("'' cannot name a task"), "{message}");
    assert!(message.contains("--name"), "{message}");
}

#[test]
fn focus() {
    let fixture = Fixture::new();
    fixture.write(
        &format!("home/{POOL}"),
        r#"[{"name":"empty","vocabulary":[],"example":[],"holdout":[],"reference":null,"goal":null}]"#,
    );
    let output = execute(&[
        "train",
        "--home",
        &fixture.home(),
        "--task",
        "empty",
        "--synthetic",
        "0",
        "--grow",
        "0",
        "--duration",
        "1",
        "--device",
        "cpu",
    ]);
    assert!(!output.status.success());
    assert!(flat(&output.stdout).contains("skipped: empty has no examples"));
    assert!(flat(&output.stderr).contains("can be trained on"));
}

#[test]
fn copy() {
    let fixture = Fixture::new();
    let home = fixture.home();
    let input = fixture.write("input.wave", "A");
    let output = fixture.write("output.wave", "B");
    succeed(&[
        "solve", "--home", &home, "--name", "rename", "--input", &input, "--output", &output,
        "--size", "0.2",
    ]);
    let report = succeed(&[
        "solve",
        "--home",
        &home,
        "--task",
        "rename",
        "--processor",
        "2",
    ]);
    assert!(report.contains("rename.p2.s0.2: optimal cost"), "{report}");
}

#[test]
fn guided() {
    let fixture = Fixture::new();
    let home = fixture.home();
    let shape = Shape {
        width: DIMENSION,
        depth: 1,
        head: 1,
        hidden: 32,
        key: DIMENSION,
    };
    let model = Model::new(shape.architecture(), &mut Generator::new(1));
    let optimizer = Optimizer::new(model.size(), optimizer::Setting::default());
    checkpoint::save(
        &fixture.path.join("home").join(CHECKPOINT),
        &model,
        &optimizer,
    )
    .unwrap();
    let input = fixture.write("input.wave", "A");
    let output = fixture.write("output.wave", "B");
    let report = succeed(&[
        "solve", "--home", &home, "--name", "rename", "--input", &input, "--output", &output,
        "--size", "0", "--guide", "60",
    ]);
    assert!(
        report.contains("rename: without a positive size weight"),
        "{report}"
    );
    assert!(
        report.contains("rename: guided search found cost 1.250"),
        "{report}"
    );
    assert!(
        report.contains("rename: optimal, because every correct program costs at least 1.250"),
        "{report}"
    );
}
