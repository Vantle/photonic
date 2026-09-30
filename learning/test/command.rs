use learning::encoding::{DIMENSION, Shape};
use learning::home::{ARCHIVE, CHECKPOINT, POOL};
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
fn range() {
    for argument in [
        ["train", "--focus=1.5"],
        ["train", "--infer=NaN"],
        ["curriculum", "--mastery=2"],
        ["curriculum", "--rehearse=-0.1"],
    ] {
        assert!(fail(&argument).contains("a share must be"), "{argument:?}");
    }
    for argument in [
        ["train", "--game=0"],
        ["improve", "--worker=0"],
        ["train", "--trainer=0"],
        ["train", "--simulation=0"],
        ["train", "--considered=0"],
        ["train", "--step=0"],
        ["train", "--depth=0"],
        ["train", "--hidden=0"],
        ["curriculum", "--exam=0"],
    ] {
        assert!(fail(&argument).contains("at least 1"), "{argument:?}");
    }
    for argument in [["improve", "--frozen"], ["curriculum", "--frozen"]] {
        assert!(fail(&argument).contains("--frozen"), "{argument:?}");
    }
    let help = succeed(&["--help"]);
    for text in [
        "Train the network by self-play",
        "Search for the cheapest programs",
        "Teach levels of increasing complexity",
    ] {
        assert!(help.contains(text), "{help}");
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
        "--fresh",
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
    untrained(&fixture);
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

#[test]
fn deadline() {
    let fixture = Fixture::new();
    let input = fixture.write("input.wave", "X.([A] B)");
    let output = fixture.write("output.wave", "C");
    let report = succeed(&[
        "solve",
        "--home",
        &fixture.home(),
        "--input",
        &input,
        "--output",
        &output,
        "--enumerate",
        "1",
    ]);
    assert!(
        report.contains("found programs for 0 of 1 tasks"),
        "{report}"
    );
}

const TASK: &str = r#"{"name":"NAME","vocabulary":["A","B"],"example":[{"input":[[{"Atom":0}]],"output":[[{"id":0,"value":{"Atom":1}}]]}],"holdout":[],"reference":null,"goal":null}"#;

const RECORD: &str = r#"{"program":{"rule":[{"input":[[{"Atom":5}]],"output":[{"Particle":[{"Atom":1}]}]}]},"cost":1.0,"correctness":1.0,"time":0.0,"work":0.0,"span":0.0,"size":4,"verified":true,"general":true,"proof":null,"moment":0}"#;

#[test]
fn stale() {
    let fixture = Fixture::new();
    let home = fixture.home();
    let task = |name: &str| TASK.replace("NAME", name);
    fixture.write(
        &format!("home/{POOL}"),
        &format!("[{},{}]", task("stale"), task("fresh")),
    );
    fixture.write(
        &format!("home/{ARCHIVE}"),
        &format!(r#"{{"stale":{{"best":{RECORD},"partial":null,"baseline":2.0}}}}"#),
    );
    let message = "names atoms outside the task's vocabulary";
    let status = succeed(&["status", "--home", &home, "--task", "stale"]);
    assert!(status.contains(message), "{status}");
    let verify = succeed(&["verify", "--home", &home, "--task", "stale"]);
    assert!(verify.contains(message), "{verify}");
    let report = succeed(&["solve", "--home", &home, "--task", "fresh"]);
    assert!(
        report.contains("found programs for 1 of 1 tasks"),
        "{report}"
    );
    assert!(fixture.path.join("home/program/fresh.wave").exists());
    assert!(!fixture.path.join("home/program/stale.wave").exists());
}

#[test]
fn malformed() {
    let fixture = Fixture::new();
    fixture.write(
        &format!("home/{POOL}"),
        &format!(
            "[{}]",
            TASK.replace("NAME", "wide")
                .replace(r#"{"Atom":1}"#, r#"{"Atom":7}"#)
        ),
    );
    let message = fail(&["verify", "--home", &fixture.home()]);
    assert!(message.contains("pool.json is malformed"), "{message}");
    assert!(
        message.contains("wide names an atom outside its vocabulary of 2"),
        "{message}"
    );
}

#[test]
fn forever() {
    let fixture = Fixture::new();
    let home = fixture.home();
    let input = fixture.write("input.wave", "A");
    let output = fixture.write("output.wave", "B");
    let maximum = u64::MAX.to_string();
    succeed(&[
        "curriculum",
        "--home",
        &home,
        "--duration",
        &maximum,
        "--level",
        "0",
    ]);
    succeed(&[
        "improve",
        "--home",
        &home,
        "--duration",
        &maximum,
        "--round",
        "0",
    ]);
    let report = succeed(&[
        "solve",
        "--home",
        &home,
        "--input",
        &input,
        "--output",
        &output,
        "--enumerate",
        &maximum,
    ]);
    assert!(
        report.contains("found programs for 1 of 1 tasks"),
        "{report}"
    );
}

fn untrained(fixture: &Fixture) {
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
}

#[test]
fn rest() {
    let fixture = Fixture::new();
    let home = fixture.home();
    let rounds = succeed(&[
        "improve",
        "--home",
        &home,
        "--round",
        "2",
        "--practice",
        "0",
        "--fresh",
        "1",
        "--guide",
        "0",
        "--device",
        "cpu",
    ]);
    assert!(rounds.contains("round 2: solved"), "{rounds}");
    let blank = Fixture::new();
    let message = fail(&["curriculum", "--home", &blank.home(), "--practice", "0"]);
    assert!(message.contains("holds none"), "{message}");
    untrained(&fixture);
    let exam = succeed(&[
        "curriculum",
        "--home",
        &home,
        "--practice",
        "0",
        "--level",
        "1",
        "--exam",
        "1",
        "--fresh",
        "0",
        "--mastery",
        "0",
        "--expansion",
        "8",
        "--enumerate",
        "60",
        "--device",
        "cpu",
    ]);
    assert!(exam.contains("round 1: level 1"), "{exam}");
    assert!(exam.contains("level 1 mastered"), "{exam}");
}

#[test]
fn builtin() {
    let fixture = Fixture::new();
    let report = succeed(&[
        "solve",
        "--home",
        &fixture.home(),
        "--task",
        "boolean.and",
        "--limit",
        "2",
    ]);
    assert!(report.contains("of 1 tasks"), "{report}");
    let pool = std::fs::read_to_string(fixture.path.join("home").join(POOL)).unwrap();
    assert!(pool.contains("boolean.and"));
}
