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

fn execute(operation: &str, path: &Path, argument: &[&str]) -> Output {
    let binary = std::env::var_os("PHOTONIC_COMMAND").expect("Photonic runfile path");
    let binary = runfiles::Runfiles::create()
        .expect("Bazel runfiles")
        .rlocation_from(binary, "")
        .expect("Photonic executable");
    Command::new(binary)
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
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.starts_with("Closed:"));
    assert!(text.contains("supported"));
    assert!(text.contains("@root"));
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
    let result = report(&execute(
        "prism",
        &path,
        &[
            "--target",
            target.to_str().unwrap(),
            "--json",
            "--work",
            "0",
        ],
    ));
    assert_eq!(result["outcome"], "unknown");
    let invalid = fixture.write("invalid.wave", "B, [B] A");
    assert!(
        execute("prism", &path, &["--target", invalid.to_str().unwrap()])
            .status
            .success()
    );
    let output = execute("prism", &path, &["--target", target.to_str().unwrap()]);
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
    let result = report(&execute(
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
    ));
    assert_eq!(result["outcome"], "unknown");
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
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("{⟨[A] B⟩@f0}@root")
    );
    let target = fixture.write("target.wave", "B");
    let complete = fixture.write("complete.wave", "B, [A] B");
    for (target, expected) in [(&target, "unreachable"), (&complete, "reached")] {
        assert_eq!(
            report(&execute(
                "prism",
                &source,
                &["--target", target.to_str().unwrap(), "--json"]
            ))["outcome"],
            expected
        );
    }
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
    assert!(String::from_utf8_lossy(&held.stdout).contains("holds   s8 by e1 e7 e8"));
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
    let cause = execute("cause", &path, &["s11.o1"]);
    assert!(String::from_utf8_lossy(&cause.stdout).contains("the scope receives True as witness"));
    let inspect = envelope(&execute("inspect", &path, &["e12", "--json"]));
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

fn session(fixture: &Fixture, line: &[impl AsRef<[u8]>]) -> Vec<serde_json::Value> {
    use std::io::Write;
    let binary = std::env::var_os("PHOTONIC_COMMAND").expect("Photonic runfile path");
    let binary = runfiles::Runfiles::create()
        .expect("Bazel runfiles")
        .rlocation_from(binary, "")
        .expect("Photonic executable");
    let mut child = Command::new(binary)
        .arg("mcp")
        .env("BUILD_WORKING_DIRECTORY", &fixture.path)
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
        &fixture,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "cause", "arguments": {"program": {"file": ["bug.wave"]}, "handle": "s11.o1"}}}),
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
    assert_eq!(legacy[5]["error"]["code"], -32602);
    assert_eq!(legacy[6]["error"]["code"], -32002);
    let meta = json!({"io.modelcontextprotocol/protocolVersion": "2026-07-28", "io.modelcontextprotocol/clientCapabilities": {}});
    let modern = session(
        &fixture,
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
        &fixture,
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
