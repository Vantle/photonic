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
        let base = std::env::var_os("TEST_TMPDIR").map_or_else(std::env::temp_dir, PathBuf::from);
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = base.join(format!(
            "photonic-{}-{time}-{}",
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

fn runfile(variable: &str) -> PathBuf {
    let name = std::env::var_os(variable).expect("a runfile path");
    runfiles::Runfiles::create()
        .expect("Bazel runfiles")
        .rlocation_from(name, "")
        .expect("a runfile")
}

fn binary() -> PathBuf {
    runfile("PHOTONIC_COMMAND")
}

fn invoke(argument: &[&str]) -> Output {
    Command::new(binary())
        .args(argument)
        .output()
        .expect("execute Photonic")
}

fn execute(operation: &str, path: &Path, argument: &[&str]) -> Output {
    Command::new(binary())
        .arg(operation)
        .arg(path)
        .args(argument)
        .output()
        .expect("execute Photonic")
}

fn report(output: &Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON execution report")
}

// A report that exits as it may, for verdicts that are not reached.
fn verdict(output: &Output, code: i32) -> serde_json::Value {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON execution report")
}

fn text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn error(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn execution() {
    let fixture = Fixture::new();
    let path = fixture.write("program.wave", "Seed.A, [Seed] ().([A] B)");
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
            .map(|token| token["atom"].as_str().unwrap_or("rule"))
            .collect::<Vec<_>>();
        particle.sort();
        particle == ["B", "Seed"]
    }));
    let output = execute("run", &path, &[]);
    assert!(output.status.success());
    let listing = text(&output);
    let line = listing.lines().collect::<Vec<_>>();
    assert!(line[0].contains(" · closed · 4 configurations · 4 events, 1 inferred"));
    assert_eq!(
        line[1..],
        [
            "s0    Seed.A",
            "s1    Seed.B",
            "s2    A.([A] B)",
            "s3    B.([A] B)"
        ]
    );
    let output = execute(
        "run",
        &path,
        &[
            "--json",
            "--work",
            "2",
            "--configuration",
            "1",
            "--occurrence",
            "5",
            "--scope",
            "3",
            "--coherence",
            "2",
        ],
    );
    let result = report(&output);
    assert_eq!(result["closed"], false);
    assert_eq!(result["limit"]["configuration"], 1);
    assert_eq!(result["limit"]["occurrence"], 5);
    assert_eq!(result["limit"]["scope"], 3);
    assert_eq!(result["limit"]["coherence"], 2);
    assert!(result["work"].as_u64().unwrap() <= 2);
}

#[test]
fn format() {
    let fixture = Fixture::new();
    let source = r#"{"initial":[["A"]],"rule":[{"input":[["A"]],"output":[["B"]]}]}"#;
    let path = fixture.write("program.json", source);
    let lower = report(&execute("run", &path, &["--json"]));
    let path = fixture.write("program.JSON", source);
    let upper = report(&execute("run", &path, &["--json"]));
    assert_eq!(lower, upper);
    assert_eq!(lower["closed"], true);
    let native = report(&execute(
        "run",
        &fixture.write("native.wave", "A, [A] B"),
        &["--json"],
    ));
    assert_eq!(native["closed"], true);
    let path = fixture.write("broken.json", "{");
    let output = execute("run", &path, &[]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("broken.json: not an assembled program")
    );
}

#[test]
fn diagnostic() {
    let fixture = Fixture::new();
    let path = fixture.write("invalid.wave", "人]");
    let output = execute("run", &path, &[]);
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.starts_with("error[source]: "), "{error}");
    assert!(
        error.contains("invalid.wave:1:2: invalid Photonic syntax"),
        "{error}"
    );
    let output = execute("run", &fixture.path.join("missing.wave"), &[]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("error[file]:"));
}

#[test]
fn worker() {
    let fixture = Fixture::new();
    let path = fixture.write("program.wave", "Seed.A, [Seed] ().([A] B)");
    let sequential = report(&execute("run", &path, &["--json"]));
    let parallel = report(&execute("run", &path, &["--json", "--worker", "4"]));
    assert_eq!(sequential, parallel);
    let paused = report(&execute("run", &path, &["--json", "--record", "1"]));
    assert_eq!(paused["closed"], false);
    assert_eq!(paused["work"], 0);
    let invalid = execute("run", &path, &["--worker", "0"]);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("--worker"));
    let listing = execute("run", &path, &["--worker", "4"]);
    assert_eq!(listing.status.code(), Some(2));
    assert!(error(&listing).contains("--json"), "{}", error(&listing));
}

// The flags that explore every plain schedule on metal, then the others.
fn plain<'text>(flag: &[&'text str]) -> Vec<&'text str> {
    [&["--plain", "--engine", "metal"][..], flag].concat()
}

// Metal explores every plain schedule through a program's net of parts, on the host in Bazel's
// sandbox, and answers explore and check from counts, ends and cycles alone.
#[test]
fn metal() {
    let fixture = Fixture::new();
    let path = fixture.write(
        "task.wave",
        "Task.T1.Pending, Task.T2.Pending, [Pending] Running, [Running] Done",
    );
    let explored = report(&execute("explore", &path, &plain(&["--json"])));
    let answer = &explored["answer"];
    assert_eq!(answer["engine"], "metal");
    assert_eq!(answer["complete"], true);
    assert_eq!(answer["configuration"], 9);
    assert_eq!(answer["event"], 12);
    assert_eq!(answer["endless"], false);
    assert_eq!(answer["end"].as_array().unwrap().len(), 1);
    assert!(answer["end"][0].get("handle").is_none());
    let done = [
        "--end",
        "Task.T1.Done, Task.T2.Done",
        "--exact",
        "--preserve",
    ];
    let passed = execute("check", &path, &plain(&done));
    assert!(passed.status.success());
    let text = String::from_utf8_lossy(&passed.stdout);
    assert!(text.contains("end Task.T1.Done, Task.T2.Done exactly   holds"));
    assert!(text.contains(" · plain · metal · 9 configurations"));
    let stray = [
        "--end",
        "Task.T1.Done, Task.T2.Running",
        "--exact",
        "--preserve",
    ];
    let failed = execute("check", &path, &plain(&stray));
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stdout).contains("a run ends at"));
    let dial = fixture.write(
        "dial.wave",
        "Dial.D1.Zero, [Zero] One, [One] Two, [Two] Zero",
    );
    let endless = execute("check", &dial, &plain(&["--end", "Zero"]));
    assert!(!endless.status.success());
    assert!(String::from_utf8_lossy(&endless.stdout).contains("a run can go on forever"));
    let open = execute(
        "check",
        &dial,
        &plain(&["--end", "Zero", "--configuration", "2", "--json"]),
    );
    let open = serde_json::from_slice::<serde_json::Value>(&open.stdout).unwrap();
    assert_eq!(open["answer"]["claim"][0]["answer"], "unknown");
    assert_eq!(open["answer"]["summary"]["complete"], false);
    let reach = execute("check", &path, &plain(&["--reach", "Done"]));
    assert!(String::from_utf8_lossy(&reach.stderr).contains("answers outcome and end"));
    let step = execute("step", &path, &plain(&[]));
    assert!(!step.status.success());
    assert!(
        String::from_utf8_lossy(&step.stderr).contains("metal keeps only counts, ends and cycles")
    );
    let exhaustive = execute("explore", &path, &["--engine", "metal"]);
    assert_eq!(exhaustive.status.code(), Some(2));
    assert!(
        error(&exhaustive).contains("--plain"),
        "{}",
        error(&exhaustive)
    );
    let run = execute("run", &path, &["--engine", "metal"]);
    assert_eq!(run.status.code(), Some(1));
    assert!(error(&run).contains("--plain --engine metal"));
}

#[test]
fn prism() {
    let fixture = Fixture::new();
    let path = fixture.write("program.wave", "A, [A] B");
    let target = fixture.write("target.particle", "B, [A] B");
    let result = report(&execute(
        "prism",
        &path,
        &["--target", target.to_str().unwrap(), "--json"],
    ));
    assert_eq!(result["outcome"], "reached");
    assert!(result["witness"].is_u64());
    assert_eq!(result["program"]["initial"][0][0], "A");
    assert_eq!(result["target"]["initial"][0][0], "B");
    let result = verdict(
        &execute(
            "prism",
            &path,
            &[
                "--target",
                target.to_str().unwrap(),
                "--json",
                "--work",
                "0",
            ],
        ),
        1,
    );
    assert_eq!(result["outcome"], "unknown");
    let invalid = fixture.write("invalid.wave", "B, [B] A");
    let output = execute("prism", &path, &["--target", invalid.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1));
    assert!(text(&output).starts_with("unreachable   "));
    let output = execute("prism", &path, &["--target", target.to_str().unwrap()]);
    assert!(output.status.success());
    assert!(text(&output).starts_with("reached   s1 by e0   B\n"));
}

// A listing's configurations by handle, after the summary that names its exploration.
fn listing(output: &Output) -> Vec<String> {
    assert!(output.status.success(), "{}", error(output));
    text(output).lines().skip(1).map(str::to_owned).collect()
}

// Both engines explore the same configurations, and run numbers them as every question does, so
// they list the same handles; prism names its witness by the same handle on either.
#[test]
fn engine() {
    let fixture = Fixture::new();
    let path = fixture.write("program.wave", "Seed.A, [Seed] ().([A] B)");
    let laser = ["--engine", "laser"];
    let interpreter = ["--engine", "interpreter"];
    assert_eq!(
        listing(&execute("run", &path, &interpreter)),
        listing(&execute("run", &path, &laser))
    );
    assert_eq!(
        listing(&execute("run", &path, &laser)),
        listing(&execute("run", &path, &[]))
    );
    let original = report(&execute(
        "run",
        &path,
        &["--json", "--engine", "interpreter"],
    ));
    let compiled = report(&execute("run", &path, &["--json", "--engine", "laser"]));
    assert_eq!(compiled["closed"], true);
    assert_eq!(compiled["definition"], original["definition"]);
    assert_eq!(compiled["limit"], original["limit"]);
    for field in ["state", "event"] {
        assert_eq!(
            compiled[field].as_array().unwrap().len(),
            original[field].as_array().unwrap().len()
        );
    }
    let parallel = report(&execute(
        "run",
        &path,
        &["--json", "--engine", "laser", "--worker", "4"],
    ));
    assert_eq!(compiled, parallel);
    let paused = report(&execute(
        "run",
        &path,
        &["--json", "--engine", "laser", "--record", "1"],
    ));
    assert_eq!(paused["closed"], false);
    assert_eq!(paused["work"], 0);
    let path = fixture.write("prism.wave", "A, [A] B");
    let target = fixture.write("target.particle", "B, [A] B");
    let missing = fixture.write("missing.particle", "C, [A] B");
    for (target, argument, outcome, code) in [
        (&target, &[][..], "reached", 0),
        (&missing, &[][..], "unreachable", 1),
        (&target, &["--work", "0"][..], "unknown", 1),
    ] {
        let argument = [
            &["--target", target.to_str().unwrap(), "--json"][..],
            argument,
        ]
        .concat();
        let original = verdict(
            &execute("prism", &path, &[&argument, &interpreter[..]].concat()),
            code,
        );
        let compiled = verdict(
            &execute("prism", &path, &[&argument, &laser[..]].concat()),
            code,
        );
        assert_eq!(original["outcome"], outcome);
        assert_eq!(compiled["outcome"], outcome);
        assert_eq!(compiled["witness"].is_u64(), outcome == "reached");
        assert_eq!(compiled["target"], original["target"]);
    }
    let summary = text(&execute("explore", &path, &[]))
        .lines()
        .next()
        .map(str::to_owned);
    for engine in [&laser, &interpreter] {
        let output = execute(
            "prism",
            &path,
            &[&["--target", target.to_str().unwrap()][..], &engine[..]].concat(),
        );
        assert!(output.status.success());
        assert!(text(&output).starts_with("reached   s1 by e0   B\n"));
    }
    let output = execute("prism", &path, &["--target", target.to_str().unwrap()]);
    assert_eq!(text(&output).lines().nth(1).map(str::to_owned), summary);
    let conflict = execute(
        "prism",
        &path,
        &[
            "--target",
            target.to_str().unwrap(),
            "--path",
            "--engine",
            "laser",
        ],
    );
    assert!(!conflict.status.success());
    assert!(String::from_utf8_lossy(&conflict.stderr).contains("--engine"));
}

// run and prism explore every schedule of plain events on Laser with --plain, as the questions do;
// inference lets a rule consume Claim early, so the full exploration holds more configurations.
#[test]
fn schedule() {
    let fixture = Fixture::new();
    let path = fixture.write("early.wave", "Claim, [Claim] P.Work, [Work] Done, [P] X");
    let full = report(&execute("run", &path, &["--json", "--engine", "laser"]));
    let schedule = report(&execute("run", &path, &["--json", "--plain"]));
    assert_eq!(schedule["closed"], true);
    assert!(schedule["state"].as_array().unwrap().len() < full["state"].as_array().unwrap().len());
    let refused = execute("run", &path, &["--plain", "--engine", "interpreter"]);
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("--plain runs on laser"));
    let target = fixture.write(
        "done.particle",
        "P.Done, [Claim] P.Work, [Work] Done, [P] X",
    );
    let reached = report(&execute(
        "prism",
        &path,
        &["--target", target.to_str().unwrap(), "--plain", "--json"],
    ));
    assert_eq!(reached["outcome"], "reached");
    let both = execute(
        "prism",
        &path,
        &["--target", target.to_str().unwrap(), "--plain", "--path"],
    );
    assert!(!both.status.success());
}

#[test]
fn depth() {
    let fixture = Fixture::new();
    for (source, code) in [
        ("[".repeat(10_000), "nesting exceeds 128 levels"),
        ("[A] ".repeat(10_000), "expansion exceeds"),
    ] {
        let path = fixture.write("deep.wave", &source);
        let output = execute("run", &path, &["--work", "0"]);
        assert!(!output.status.success());
        assert!(output.status.code().is_some());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(code),
            "{code}"
        );
    }
}

#[test]
fn group() {
    let fixture = Fixture::new();
    let path = fixture.write(
        "group.wave",
        "Add.(Unit.Unit.Unit,Unit.Unit.Unit.Unit.Unit.Unit.Unit), [Add,Add] ()",
    );
    let target = fixture.write(
        "target.particle",
        &format!("{}, [Add,Add] ()", ["Unit"; 10].join(".")),
    );
    let argument = ["--target", target.to_str().unwrap(), "--json"];
    let result = report(&execute("prism", &path, &argument));
    assert_eq!(result["outcome"], "reached");
    let parallel = report(&execute(
        "prism",
        &path,
        &[
            "--target",
            target.to_str().unwrap(),
            "--json",
            "--worker",
            "4",
        ],
    ));
    assert_eq!(result, parallel);
    let path = fixture.write("expansion.wave", &format!("A{}", ".(B,C)".repeat(40)));
    let output = execute("run", &path, &["--work", "0"]);
    assert!(!output.status.success());
    assert!(output.status.code().is_some());
    assert!(String::from_utf8_lossy(&output.stderr).contains("expansion exceeds"));
}

#[test]
fn path() {
    let fixture = Fixture::new();
    let source = fixture.write("program.wave", "A, [A] B, [B] C");
    let target = fixture.write("target.particle", "C, [A] B, [B] C");
    let result = report(&execute(
        "prism",
        &source,
        &["--target", target.to_str().unwrap(), "--path", "--json"],
    ));
    assert_eq!(result["outcome"], "reached");
    assert_eq!(result["event"].as_array().unwrap().len(), 2);
    let result = verdict(
        &execute(
            "prism",
            &source,
            &[
                "--target",
                target.to_str().unwrap(),
                "--path",
                "--json",
                "--work",
                "0",
            ],
        ),
        1,
    );
    assert_eq!(result["outcome"], "unknown");
    let reached = execute(
        "prism",
        &source,
        &["--target", target.to_str().unwrap(), "--path"],
    );
    assert!(reached.status.success());
    let line = text(&reached)
        .lines()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(line[0], "reached   s2 by e0 e1   C");
    assert!(
        line[1].contains(" · path reached its goal · 3 configurations · 2 events, 0 inferred"),
        "{}",
        line[1]
    );
    let goal = execute("explore", &source, &["--path", "--goal", "C, [A] B, [B] C"]);
    assert_eq!(text(&goal).lines().next(), Some(line[1].as_str()));
    let step = fixture.write("step.particle", "B, [A] B, [B] C");
    let single = execute(
        "prism",
        &source,
        &["--target", step.to_str().unwrap(), "--path"],
    );
    assert!(text(&single).contains(" · 1 event, "), "{}", text(&single));
    let stopped = execute(
        "prism",
        &source,
        &[
            "--target",
            target.to_str().unwrap(),
            "--path",
            "--work",
            "0",
        ],
    );
    assert_eq!(stopped.status.code(), Some(1));
    assert!(text(&stopped).starts_with("unknown   a direct path follows one run of many"));
}

#[test]
fn extension() {
    let fixture = Fixture::new();
    let source = "A, [A] B";
    let target = fixture.write("target.particle", "B, [A] B");
    let mut expected = None;
    for name in ["program.particle", "program.wave", "program.WAVE"] {
        let path = fixture.write(name, source);
        let actual = report(&execute("run", &path, &["--json"]));
        if let Some(expected) = &expected {
            assert_eq!(&actual, expected);
        } else {
            expected = Some(actual);
        }
        let proof = report(&execute(
            "prism",
            &path,
            &["--target", target.to_str().unwrap(), "--json"],
        ));
        assert_eq!(proof["outcome"], "reached");
    }
}

#[test]
fn library() {
    let fixture = Fixture::new();
    let path = fixture.write("program.wave", "Call.Not.True");
    let target = fixture.write(
        "target.particle",
        "False, [Call.Not.True] Return.False, [Return] ()",
    );
    let library = fixture.write("boolean.particle", "[Call.Not.True] Return.False");
    let completion = fixture.write("completion.particle", "[Return] ()");
    let output = execute(
        "prism",
        &path,
        &[
            "--target",
            target.to_str().unwrap(),
            "--library",
            library.to_str().unwrap(),
            "--library",
            completion.to_str().unwrap(),
            "--json",
        ],
    );
    let result = report(&output);
    assert_eq!(result["outcome"], "reached");
    assert_eq!(result["execution"]["closed"], true);
    let output = execute(
        "prism",
        &path,
        &[
            "--target",
            target.to_str().unwrap(),
            "--library",
            library.to_str().unwrap(),
            "--library",
            completion.to_str().unwrap(),
            "--path",
            "--json",
        ],
    );
    assert_eq!(report(&output)["outcome"], "reached");
    let output = execute(
        "run",
        &path,
        &["--library", library.to_str().unwrap(), "--json"],
    );
    assert_eq!(report(&output)["closed"], true);
    let invalid = fixture.write("invalid.particle", "Unexpected");
    let output = execute("run", &path, &["--library", invalid.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("error[library]:"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let malformed = fixture.write("malformed.particle", "[");
    let output = execute("run", &path, &["--library", malformed.to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("malformed.particle"));
}

#[test]
fn serialization() {
    let fixture = Fixture::new();
    let path = fixture.write("program.wave", "[] A");
    let normal = execute("run", &path, &["--json", "--work", "100"]);
    let compact = execute("run", &path, &["--json", "--compact", "--work", "100"]);
    assert_eq!(report(&normal), report(&compact));
    assert!(compact.stdout.len() < normal.stdout.len());
    let lowered = report(&execute("lower", &path, &[]));
    assert_eq!(lowered["initial"], serde_json::json!([]));
    assert_eq!(lowered["rule"][0]["input"], serde_json::json!([]));
    assert_eq!(lowered["rule"][0]["output"][0], serde_json::json!(["A"]));
    let path = fixture.write("scope.wave", "Z, (X, [X] Y), [A] (B, C, [B, C] D)");
    let lowered = report(&execute("lower", &path, &[]));
    assert_eq!(
        lowered["scope"],
        serde_json::json!([{"initial": [["X"]], "rule": [{"name": "[X] Y", "input": [["X"]], "output": [["Y"]]}]}])
    );
    assert_eq!(
        lowered["rule"][0]["output"][0]["initial"],
        serde_json::json!([["B"], ["C"]])
    );
}

#[test]
fn context() {
    let fixture = Fixture::new();
    let source = fixture.write("source.wave", "A, [A] B");
    let declaration = fixture.write("rule.wave", "[A] B");
    let output = execute("run", &declaration, &[]);
    assert!(output.status.success());
    assert_eq!(listing(&output), ["s0    nothing"]);
    let target = fixture.write("target.wave", "B");
    let complete = fixture.write("complete.wave", "B, [A] B");
    for (target, expected, code) in [(&target, "unreachable", 1), (&complete, "reached", 0)] {
        assert_eq!(
            verdict(
                &execute(
                    "prism",
                    &source,
                    &["--target", target.to_str().unwrap(), "--json"]
                ),
                code
            )["outcome"],
            expected
        );
    }
}

// --preserve adds every loaded root rule to prism's target, as photonic_test does, in every mode.
#[test]
fn preservation() {
    let fixture = Fixture::new();
    let source = fixture.write("source.wave", "A, [A] B");
    let library = fixture.write("library.particle", "[B] C");
    let target = fixture.write("target.wave", "C");
    let argument = |extra: &[&'static str]| {
        [
            &[
                "--target",
                target.to_str().unwrap(),
                "--library",
                library.to_str().unwrap(),
            ][..],
            extra,
        ]
        .concat()
    };
    for mode in [
        &[][..],
        &["--plain"],
        &["--engine", "interpreter"],
        &["--path"],
    ] {
        let bare = execute("prism", &source, &argument(mode));
        assert_eq!(bare.status.code(), Some(1), "{mode:?}: {}", text(&bare));
        let preserved = execute(
            "prism",
            &source,
            &argument(&[mode, &["--preserve"]].concat()),
        );
        assert!(preserved.status.success(), "{mode:?}: {}", text(&preserved));
        assert!(text(&preserved).starts_with("reached   "));
        let report = verdict(
            &execute(
                "prism",
                &source,
                &argument(&[mode, &["--preserve", "--json"]].concat()),
            ),
            0,
        );
        assert_eq!(report["outcome"], "reached");
        assert_eq!(report["target"]["rule"].as_array().map(Vec::len), Some(2));
    }
    let check = execute(
        "check",
        &source,
        &[
            "--reach",
            "C",
            "--exact",
            "--preserve",
            "--library",
            library.to_str().unwrap(),
        ],
    );
    let witness = text(&check)
        .lines()
        .nth(1)
        .and_then(|line| line.split("holds   ").nth(1))
        .and_then(|line| line.split("   ").next())
        .map(str::to_owned)
        .expect("a witness");
    let prism = execute("prism", &source, &argument(&["--preserve"]));
    assert!(
        text(&prism).starts_with(&format!("reached   {witness}   C\n")),
        "{witness}: {}",
        text(&prism)
    );
}

const BUG: &str = "And.True.False.Extra,
[True] Boolean,
[False] Boolean,
[And.Boolean.Boolean] (
    [True.True] True,
    [False] False,
)
";

const ORIGINAL: &str = "And.True.False.Extra,
[True] Boolean,
[False] Boolean,
[And.Boolean.Boolean] (
    [True.True] True,
    [True.False] False,
    [False.False] False,
)
";

fn envelope(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).expect("a JSON envelope")
}

#[test]
fn check() {
    let fixture = Fixture::new();
    let path = fixture.write("bug.wave", BUG);
    let held = execute(
        "check",
        &path,
        &["--reach", "False.Extra", "--exact", "--preserve"],
    );
    assert!(held.status.success());
    assert!(String::from_utf8_lossy(&held.stdout).contains("holds   s13 by e0 e7 e19"));
    let failed = execute("check", &path, &["--reach", "Nothing"]);
    assert!(!failed.status.success());
    let broken = fixture.write("broken.wave", "A B");
    let diagnostic = envelope(&execute("check", &broken, &["--json"]));
    assert_eq!(diagnostic["answer"]["diagnostic"][0]["code"], "syntax");
}

#[test]
fn question() {
    let fixture = Fixture::new();
    let path = fixture.write("bug.wave", BUG);
    let explored = envelope(&execute("explore", &path, &["--json"]));
    assert_eq!(explored["answer"]["configuration"], 18);
    let cause = execute("cause", &path, &["s6.o1"]);
    assert!(String::from_utf8_lossy(&cause.stdout).contains("the scope receives True as witness"));
    let inspect = envelope(&execute("inspect", &path, &["e11", "--json"]));
    assert_eq!(inspect["answer"]["kind"], "event");
    let select = envelope(&execute(
        "select",
        &path,
        &["--pattern", "[False] False", "--json"],
    ));
    assert_eq!(select["answer"]["total"], 3);
    let miss = envelope(&execute("miss", &path, &["r3", "--json"]));
    assert_eq!(miss["answer"]["kind"], "rule");
    let step = envelope(&execute("step", &path, &["--json"]));
    assert_eq!(step["answer"]["handle"], "s0");
    let missing = execute("inspect", &path, &["s99"]);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("error[handle]"));
}

#[test]
fn spectrum() {
    let fixture = Fixture::new();
    let path = fixture.write("bug.wave", BUG);
    let compiled = envelope(&execute("explore", &path, &["--json"]));
    let interpreter = envelope(&execute(
        "explore",
        &path,
        &["--json", "--engine", "interpreter"],
    ));
    assert_eq!(interpreter["answer"]["engine"], "interpreter");
    assert_eq!(compiled["answer"]["engine"], "laser");
    for field in ["complete", "configuration", "event", "inferred", "depth"] {
        assert_eq!(
            compiled["answer"][field], interpreter["answer"][field],
            "{field}"
        );
    }
    assert_ne!(
        compiled["answer"]["exploration"],
        interpreter["answer"]["exploration"]
    );
    let text = execute("explore", &path, &["--engine", "interpreter"]);
    assert!(String::from_utf8_lossy(&text.stdout).contains(" · interpreter · "));
    let default = execute("explore", &path, &[]);
    assert!(!String::from_utf8_lossy(&default.stdout).contains(" · interpreter · "));
    let held = execute(
        "check",
        &path,
        &[
            "--reach",
            "False.Extra",
            "--exact",
            "--preserve",
            "--engine",
            "laser",
        ],
    );
    assert!(held.status.success());
    let failed = execute("check", &path, &["--reach", "Nothing", "--engine", "laser"]);
    assert!(!failed.status.success());
    let step = envelope(&execute("step", &path, &["--json", "--engine", "laser"]));
    assert_eq!(step["answer"]["handle"], "s0");
    let select = envelope(&execute(
        "select",
        &path,
        &["--pattern", "[False] False", "--json", "--engine", "laser"],
    ));
    assert_eq!(select["answer"]["total"], 3);
    let conflict = execute("explore", &path, &["--path", "--engine", "laser"]);
    assert!(!conflict.status.success());
    assert!(String::from_utf8_lossy(&conflict.stderr).contains("--engine"));
    let invalid = execute("explore", &path, &["--engine", "gpu"]);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("interpreter, laser and metal"));
    let early = fixture.write("early.wave", "Claim, [Claim] P.Work, [Work] Done, [P] X");
    let inferred = execute("check", &early, &["--inevitable", "Done"]);
    assert!(!inferred.status.success());
    let schedule = execute("check", &early, &["--inevitable", "Done", "--plain"]);
    assert!(schedule.status.success());
    assert!(String::from_utf8_lossy(&schedule.stdout).contains(" · plain · "));
    let interpreter = execute("check", &early, &["--plain", "--engine", "interpreter"]);
    assert!(!interpreter.status.success());
    assert!(String::from_utf8_lossy(&interpreter.stderr).contains("plain mode runs on laser"));
    let both = execute("explore", &early, &["--plain", "--path"]);
    assert!(!both.status.success());
}

#[test]
fn compare() {
    let fixture = Fixture::new();
    let original = fixture.write("original.wave", ORIGINAL);
    let bug = fixture.write("bug.wave", BUG);
    let changed = execute("compare", &original, &[bug.to_str().unwrap()]);
    assert!(!changed.status.success());
    assert!(String::from_utf8_lossy(&changed.stdout).contains("+ False.True.Extra"));
    let same = execute("compare", &original, &[original.to_str().unwrap()]);
    assert!(same.status.success());
    assert!(String::from_utf8_lossy(&same.stdout).contains("configurations   identical · 14"));
}

#[test]
fn shape() {
    let fixture = Fixture::new();
    let light = fixture.write(
        "light.wave",
        "Light, [Light] Red, [Light] Green, [Light] Blue",
    );
    let result = envelope(&execute("shape", &light, &["--json"]));
    assert_eq!(result["answer"]["form"]["size"], "6");
    let fixed = envelope(&execute("shape", &light, &["--json", "--fix", "Red"]));
    assert_eq!(fixed["answer"]["form"]["size"], "2");
    let and = fixture.write(
        "and.particle",
        "[Function.And.True.True] Return.True, [Function.And.True.False] Return.False, [Function.And.False.False] Return.False",
    );
    let or = fixture.write(
        "or.particle",
        "[Function.Or.True.True] Return.True, [Function.Or.True.False] Return.True, [Function.Or.False.False] Return.False",
    );
    let equal = fixture.write(
        "equal.particle",
        "[Function.Equal.True.True] Return.True, [Function.Equal.True.False] Return.False, [Function.Equal.False.False] Return.True",
    );
    let grouped = execute("shape", &and, &[or.to_str().unwrap()]);
    assert!(grouped.status.success());
    assert!(String::from_utf8_lossy(&grouped.stdout).contains("by And → Or (True False)"));
    let apart = execute(
        "shape",
        &and,
        &[or.to_str().unwrap(), equal.to_str().unwrap()],
    );
    assert!(!apart.status.success());
    assert!(String::from_utf8_lossy(&apart.stdout).contains("in 2 shapes"));
    let left = fixture.write("left.wave", "Seed.A, [Seed] ().([A] B), [B] C");
    let right = fixture.write("right.wave", "[Y] Z, Root.X, [Root] ().([X] Y)");
    let first = envelope(&execute("shape", &left, &["--json"]));
    let second = envelope(&execute("shape", &right, &["--json"]));
    assert_eq!(
        first["answer"]["form"]["text"],
        second["answer"]["form"]["text"]
    );
}

fn session(directory: &Path, line: &[impl AsRef<[u8]>]) -> Vec<serde_json::Value> {
    use std::io::Write;
    let mut child = Command::new(binary())
        .arg("mcp")
        .env("BUILD_WORKING_DIRECTORY", directory)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("start the server");
    {
        let mut input = child.stdin.take().expect("server input");
        for message in line {
            input.write_all(message.as_ref()).expect("send a message");
            input.write_all(b"\n").expect("end the message");
        }
    }
    let output = child.wait_with_output().expect("the server exits");
    assert!(output.status.success());
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).expect("one JSON message per line"))
        .collect()
}

#[test]
fn server() {
    use serde_json::json;
    let fixture = Fixture::new();
    fixture.write("bug.wave", BUG);
    let legacy = session(
        &fixture.path,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "cause", "arguments": {"program": {"file": ["bug.wave"]}, "handle": "s6.o1"}}}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "resources/read", "params": {"uri": "photonic://primer"}}),
            json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {"name": "inspect", "arguments": {"program": {"file": ["bug.wave"]}, "handle": "s99"}}}),
            json!({"jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": {"name": "nothing", "arguments": {}}}),
            json!({"jsonrpc": "2.0", "id": 7, "method": "resources/read", "params": {"uri": "photonic://nothing"}}),
        ]
        .map(|message| message.to_string()),
    );
    assert_eq!(legacy.len(), 7);
    assert_eq!(legacy[0]["result"]["protocolVersion"], "2025-06-18");
    let tool = legacy[1]["result"]["tools"].as_array().expect("tools");
    assert_eq!(tool.len(), 9);
    assert!(tool.iter().all(|tool| tool["outputSchema"].is_object()));
    let text = legacy[2]["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(
        text.contains("the scope receives True as witness"),
        "{text}"
    );
    assert_eq!(
        legacy[2]["result"]["structuredContent"]["kind"],
        "occurrence"
    );
    assert!(
        legacy[3]["result"]["contents"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .contains("## Grammar")
    );
    assert_eq!(legacy[4]["result"]["isError"], true);
    assert!(
        legacy[4]["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .starts_with("error[handle]: s99 names no configuration")
    );
    assert_eq!(legacy[5]["error"]["code"], -32602);
    assert_eq!(legacy[6]["error"]["code"], -32002);
    let meta = json!({"io.modelcontextprotocol/protocolVersion": "2026-07-28", "io.modelcontextprotocol/clientCapabilities": {}});
    let modern = session(
        &fixture.path,
        &[
            json!({"jsonrpc": "2.0", "id": "a", "method": "server/discover", "params": {"_meta": meta}}),
            json!({"jsonrpc": "2.0", "id": "b", "method": "tools/call", "params": {"_meta": meta, "name": "explore", "arguments": {"program": {"file": ["bug.wave"]}}}}),
            json!({"jsonrpc": "2.0", "id": "c", "method": "tools/list", "params": {"_meta": {"io.modelcontextprotocol/protocolVersion": "1900-01-01", "io.modelcontextprotocol/clientCapabilities": {}}}}),
            json!({"jsonrpc": "2.0", "id": "d", "method": "tools/list", "params": {"_meta": {"io.modelcontextprotocol/protocolVersion": "2026-07-28"}}}),
            json!({"jsonrpc": "2.0", "id": "e", "method": "resources/read", "params": {"_meta": meta, "uri": "photonic://nothing"}}),
            json!(["not", "a", "request"]),
            json!({"jsonrpc": "2.0", "id": "f", "params": {"_meta": meta}}),
            json!({"jsonrpc": "2.0", "id": "g", "method": "nothing", "params": {"_meta": meta}}),
            json!("{not json"),
            json!([{"jsonrpc": "2.0", "id": "h", "method": "ping", "params": {"_meta": meta}}, {"jsonrpc": "2.0", "method": "notifications/initialized"}]),
            json!([]),
            json!({"jsonrpc": "2.0", "method": 1}),
            json!({"jsonrpc": "2.0", "id": "i", "method": "tools/list", "params": {"_meta": {"io.modelcontextprotocol/protocolVersion": "2025-06-18"}}}),
            json!({"jsonrpc": "2.0", "id": {"nested": 1}, "method": "ping"}),
            json!({"jsonrpc": "2.0", "id": null, "method": "ping"}),
            json!({"id": "j", "method": "ping", "params": {"_meta": meta}}),
            json!({"jsonrpc": "2.0", "id": "k", "method": "ping", "params": ["_meta"]}),
        ]
        .map(|message| match message {
            serde_json::Value::String(line) => line,
            message => message.to_string(),
        }),
    );
    assert_eq!(modern[0]["result"]["resultType"], "complete");
    assert!(
        modern[0]["result"]["supportedVersions"]
            .as_array()
            .expect("versions")
            .contains(&json!("2026-07-28"))
    );
    assert_eq!(
        modern[0]["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "photonic"
    );
    assert_eq!(
        modern[1]["result"]["structuredContent"]["configuration"],
        18
    );
    assert_eq!(modern[2]["error"]["code"], -32022);
    assert_eq!(modern[2]["error"]["data"]["requested"], "1900-01-01");
    assert_eq!(modern[3]["error"]["code"], -32602);
    assert_eq!(modern[4]["error"]["code"], -32602);
    let invalid = modern[5].as_array().expect("a batch answer");
    assert_eq!(invalid.len(), 3);
    assert!(invalid.iter().all(|error| error["error"]["code"] == -32600));
    assert_eq!(modern[6]["error"]["code"], -32600);
    assert_eq!(modern[7]["error"]["code"], -32601);
    assert_eq!(modern[8]["error"]["code"], -32700);
    assert_eq!(modern[8]["id"], serde_json::Value::Null);
    let batch = modern[9].as_array().expect("a batch answer");
    assert_eq!(batch.len(), 1);
    assert_eq!(batch[0]["id"], "h");
    assert_eq!(modern[10]["error"]["code"], -32600);
    assert_eq!(modern[11]["error"]["code"], -32600);
    assert_eq!(modern[11]["id"], serde_json::Value::Null);
    assert!(modern[12]["result"]["tools"][0]["outputSchema"].is_object());
    assert!(modern[12]["result"].get("resultType").is_none());
    for index in [13, 14] {
        assert_eq!(modern[index]["error"]["code"], -32600);
        assert_eq!(modern[index]["id"], serde_json::Value::Null);
    }
    assert_eq!(modern[15]["error"]["code"], -32600);
    assert_eq!(modern[15]["id"], "j");
    assert_eq!(modern[16]["error"]["code"], -32602);
    let broken = session(
        &fixture.path,
        &[
            b"\xff\xfe".to_vec(),
            json!({"jsonrpc": "2.0", "id": 1, "method": "ping"})
                .to_string()
                .into_bytes(),
        ],
    );
    assert_eq!(broken.len(), 2);
    assert_eq!(broken[0]["error"]["code"], -32700);
    assert_eq!(broken[1]["id"], 1);
    assert!(broken[1]["result"].is_object());
}

const LIGHT: &str = "Light, [Light] Red, [Light] Green, [Light] Blue";

fn initialize() -> String {
    serde_json::json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}}}).to_string()
}

fn call(id: usize, name: &str, argument: serde_json::Value) -> String {
    serde_json::json!({"jsonrpc": "2.0", "id": id, "method": "tools/call", "params": {"name": name, "arguments": argument}}).to_string()
}

// A tool result's text, and whether it is an error.
fn result(response: &serde_json::Value) -> (bool, String) {
    let result = &response["result"];
    (
        result["isError"] == true,
        result["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
    )
}

// A goal reaches every question, --preserve needs an exact target or the goal to complete, and
// what the command line cannot mean is refused before any work.
#[test]
fn flag() {
    let fixture = Fixture::new();
    let path = fixture.write("chain.wave", "A, [A] B, [B] C");
    let other = fixture.write("other.wave", "A, [A] B, [B] C, [C] D");
    let goal = ["--path", "--goal", "C, [A] B, [B] C"];
    let inspect = execute("inspect", &path, &[&["s2"][..], &goal].concat());
    assert!(inspect.status.success(), "{}", error(&inspect));
    assert!(text(&inspect).starts_with("s2 C"));
    let cause = execute("cause", &path, &[&["s2"][..], &goal].concat());
    assert!(cause.status.success(), "{}", error(&cause));
    assert!(text(&cause).contains("e1"));
    let step = execute("step", &path, &[&["s1"][..], &goal].concat());
    assert!(step.status.success(), "{}", error(&step));
    assert!(text(&step).contains("taken"));
    let select = execute("select", &path, &[&["--pattern", "C"][..], &goal].concat());
    assert!(text(&select).contains("s2"));
    let compare = execute(
        "compare",
        &path,
        &[&[other.to_str().unwrap()][..], &goal].concat(),
    );
    let explored = text(&execute("explore", &path, &goal));
    let key = explored.split(" · ").next().unwrap_or_default();
    assert!(
        text(&compare).starts_with(&format!("compare {key} open")),
        "{}",
        text(&compare)
    );
    let preserved = execute("explore", &path, &["--path", "--goal", "C", "--preserve"]);
    assert!(text(&preserved).contains("path reached its goal"));
    for argument in [
        &["--goal", "C"][..],
        &["--preserve"],
        &["--preserve", "--reach", "C"],
        &["--worker", "2"],
        &["--engine", "metal"],
        &["--path", "--plain"],
    ] {
        let output = execute("check", &path, argument);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{argument:?}: {}",
            error(&output)
        );
    }
    let exact = execute("miss", &path, &["--exact"]);
    assert_eq!(exact.status.code(), Some(2));
    let preserve = execute("check", &path, &["--reach", "C", "--exact", "--preserve"]);
    assert!(preserve.status.success(), "{}", text(&preserve));
    let help = |verb: &str| text(&invoke(&[verb, "--help"]));
    assert!(!help("step").contains("r2"));
    assert!(help("step").contains("s0, the start, by default"));
    assert!(!help("cause").contains("s10.f1"));
    assert!(help("inspect").contains("s10.f1"));
    assert!(!help("run").contains("metal as many"));
    assert!(!help("prism").contains("metal as many"));
    assert!(help("explore").contains("metal as many as the GPU holds"));
    assert!(help("select").contains("Program files: .wave or .particle source"));
    assert!(help("compare").contains("Photonic source added after the file of each program"));
    assert!(help("shape").contains("Photonic source added after the file of each program"));
}

// Answers list claims in the order the command line gives them, whatever their kinds.
#[test]
fn order() {
    let fixture = Fixture::new();
    let path = fixture.write("light.wave", LIGHT);
    let claim = [
        "--avoid",
        "Purple",
        "--reach",
        "Red",
        "--outcome",
        "Light",
        "--reach",
        "Green",
    ];
    let output = execute("check", &path, &claim);
    let line = text(&output)
        .lines()
        .skip(1)
        .take(4)
        .map(|line| line.split("   ").next().unwrap_or_default().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        line,
        ["avoid Purple", "reach Red", "outcome Light", "reach Green"]
    );
    let answer = envelope(&execute(
        "check",
        &path,
        &[&claim[..], &["--json"]].concat(),
    ));
    let kind = answer["answer"]["claim"]
        .as_array()
        .expect("verdicts")
        .iter()
        .map(|verdict| {
            verdict["claim"]["kind"]
                .as_str()
                .unwrap_or_default()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(kind, ["avoid", "reach", "outcome", "reach"]);
    let compared = envelope(&execute(
        "compare",
        &path,
        &[&[path.to_str().unwrap()][..], &claim, &["--json"]].concat(),
    ));
    let pattern = compared["answer"]["claim"]
        .as_array()
        .expect("pairs")
        .iter()
        .map(|pair| {
            pair["claim"]["pattern"]
                .as_str()
                .unwrap_or_default()
                .to_owned()
        })
        .collect::<Vec<_>>();
    assert_eq!(pattern, ["Purple", "Red", "Light", "Green"]);
}

// Inline source names a program on its own in every command, and a command that names no program
// is a failure that says how to name one.
#[test]
fn source() {
    let fixture = Fixture::new();
    let target = fixture.write("target.wave", "B, [A] B");
    let target = target.to_str().unwrap();
    for argument in [
        &["lower", "--source", "A, [A] B"][..],
        &["run", "--source", "A, [A] B"],
        &["prism", "--source", "A, [A] B", "--target", target],
        &["explore", "--source", "A, [A] B"],
        &["step", "--source", "A, [A] B"],
        &["inspect", "--source", "A, [A] B", "s1"],
        &["shape", "--source", "A, [A] B"],
    ] {
        let output = invoke(argument);
        assert!(output.status.success(), "{argument:?}: {}", error(&output));
    }
    for argument in [
        &["run"][..],
        &["lower"],
        &["explore"],
        &["shape"],
        &["inspect", "s1"],
    ] {
        let output = invoke(argument);
        assert_eq!(output.status.code(), Some(1), "{argument:?}");
        assert!(
            error(&output).starts_with(
                "error[request]: name the program: give its files, or its text with --source"
            ),
            "{argument:?}: {}",
            error(&output)
        );
    }
    let answer = envelope(&invoke(&["check", "--json"]));
    assert_eq!(answer["error"]["code"], "request");
}

// A link inside the server's directory to a file outside it, where the platform makes links without
// privileges.
#[cfg(unix)]
fn escape(root: &Path, secret: &Path) -> Option<String> {
    std::os::unix::fs::symlink(secret, root.join("link.wave")).unwrap();
    Some("link.wave".to_owned())
}

#[cfg(not(unix))]
fn escape(_: &Path, _: &Path) -> Option<String> {
    None
}

// The protocol server reads only files inside the directory it starts in, following links; the
// command line reads any path.
#[test]
fn confinement() {
    let fixture = Fixture::new();
    let root = fixture.path.join("root");
    std::fs::create_dir(&root).unwrap();
    let secret = fixture.write("secret.wave", "Top.Secret");
    std::fs::write(root.join("light.wave"), LIGHT).unwrap();
    let inside = root.join("light.wave");
    let message = [
        initialize(),
        call(
            1,
            "explore",
            serde_json::json!({"program": {"file": ["light.wave"]}}),
        ),
        call(
            2,
            "explore",
            serde_json::json!({"program": {"file": [inside.to_str().unwrap()]}}),
        ),
        call(
            3,
            "explore",
            serde_json::json!({"program": {"file": ["../secret.wave"]}}),
        ),
        call(
            4,
            "explore",
            serde_json::json!({"program": {"file": [secret.to_str().unwrap()]}}),
        ),
        call(
            5,
            "explore",
            serde_json::json!({"program": {"file": ["light.wave"], "library": ["../secret.wave"]}}),
        ),
    ]
    .into_iter()
    .chain(escape(&root, &secret).map(|link| {
        call(
            6,
            "explore",
            serde_json::json!({"program": {"file": [link]}}),
        )
    }))
    .collect::<Vec<_>>();
    let answer = session(&root, &message);
    for response in &answer[1..3] {
        let (failed, text) = result(response);
        assert!(!failed, "{text}");
        assert!(text.contains(" · closed · 4 configurations"), "{text}");
    }
    for response in &answer[3..] {
        let (failed, text) = result(response);
        assert!(failed, "{text}");
        assert!(text.starts_with("error[file]: "), "{text}");
        assert!(
            text.contains("the server reads no file outside it"),
            "{text}"
        );
        assert!(!text.contains("Secret"), "{text}");
    }
    let output = execute("explore", &secret, &[]);
    assert!(output.status.success());
    assert!(text(&output).contains("Top.Secret") || text(&output).contains("Secret.Top"));
}

// A tool that fails says its failure's code as the command line does, and an argument of the wrong
// type is named by its path.
#[test]
fn failure() {
    let fixture = Fixture::new();
    fixture.write("light.wave", LIGHT);
    let program = serde_json::json!({"file": ["light.wave"]});
    let answer = session(
        &fixture.path,
        &[
            initialize(),
            call(
                1,
                "explore",
                serde_json::json!({"program": {"file": "light.wave"}}),
            ),
            call(
                2,
                "check",
                serde_json::json!({"program": program, "claim": [{"kind": "reach"}]}),
            ),
            call(
                3,
                "explore",
                serde_json::json!({"program": program, "mode": "linear"}),
            ),
            call(
                4,
                "explore",
                serde_json::json!({"program": program, "limit": -1}),
            ),
            call(
                5,
                "explore",
                serde_json::json!({"program": program, "budget": {"work": "many"}}),
            ),
            call(6, "inspect", serde_json::json!({"program": program})),
            call(
                7,
                "compare",
                serde_json::json!({"left": {"program": program}, "right": {"program": {"file": [1]}}}),
            ),
            call(
                8,
                "inspect",
                serde_json::json!({"program": program, "handle": "s99"}),
            ),
            call(
                9,
                "explore",
                serde_json::json!({"program": program, "colour": 1}),
            ),
        ],
    );
    let expected = [
        "error[request]: program.file takes an array, not a string",
        "error[request]: claim.0.pattern is required",
        "error[request]: mode takes exhaustive, plain or path, not \"linear\"",
        "error[request]: limit takes at least 0, not -1",
        "error[request]: budget.work takes an integer, not a string",
        "error[request]: handle is required",
        "error[request]: right.program.file.0 takes a string, not an integer",
        "error[handle]: s99 names no configuration",
        "error[request]: colour is not a field here",
    ];
    for (response, expected) in answer[1..].iter().zip(expected) {
        let (failed, text) = result(response);
        assert!(failed, "{text}");
        assert!(text.starts_with(expected), "{text}");
    }
}
