use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static INDEX: AtomicUsize = AtomicUsize::new(0);

fn executable(name: &str) -> PathBuf {
    let runfile = runfiles::Runfiles::create().unwrap();
    runfile
        .rlocation_from(std::env::var(name).unwrap(), "")
        .unwrap()
}

fn directory() -> PathBuf {
    let root = PathBuf::from(std::env::var_os("TEST_TMPDIR").unwrap());
    let path = root.join(INDEX.fetch_add(1, Ordering::Relaxed).to_string());
    std::fs::create_dir(&path).unwrap();
    path
}

#[test]
fn assembly() {
    let root = directory();
    let first = root.join("first.wave");
    let second = root.join("second.wave");
    let library = root.join("library.particle");
    let output = root.join("program.json");
    std::fs::write(&first, "First").unwrap();
    std::fs::write(&second, "Second").unwrap();
    std::fs::write(&library, "[First] Result").unwrap();
    let invoke = || {
        Command::new(executable("ASSEMBLE"))
            .arg("--source")
            .arg(&first)
            .arg("--source")
            .arg(&second)
            .arg("--library")
            .arg(&library)
            .arg("--output")
            .arg(&output)
            .output()
            .unwrap()
    };
    let result = invoke();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let program: frontend::source::Program =
        serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
    assert_eq!(program.initial.len(), 2);
    assert_eq!(program.rule.len(), 1);
    std::fs::write(&library, "Unexpected").unwrap();
    let result = invoke();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("declarations only"));
    std::fs::write(&library, "[").unwrap();
    let result = invoke();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("library.particle"));
}

#[test]
fn binary() {
    let root = directory();
    let target = root.join("target.json");
    let output = Command::new(executable("LOCAL"))
        .current_dir(&root)
        .arg("lower")
        .output()
        .unwrap();
    assert!(output.status.success());
    let source: frontend::source::Program = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!source.rule.is_empty());
    let value = frontend::source::Program {
        rule: source.rule,
        ..frontend::lowering::parse(
            "First.([Digit] 2).([Carry] 1), Second.([Digit] 0).([Borrow] 1)",
        )
        .unwrap()
    };
    std::fs::write(&target, serde_json::to_vec(&value).unwrap()).unwrap();
    let output = Command::new(executable("LOCAL"))
        .current_dir(&root)
        .args(["prism", "--target"])
        .arg(&target)
        .args([
            "--json",
            "--path",
            "--work",
            "100000",
            "--occurrence",
            "128",
            "--configuration",
            "1024",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["outcome"], "reached");
    let output = Command::new(executable("LOCAL"))
        .current_dir(&root)
        .args(["--json", "--work", "0"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["closed"], false);
    let output = Command::new(executable("LOCAL"))
        .arg("--unknown")
        .output()
        .unwrap();
    assert!(!output.status.success());
}

#[test]
fn diamond() {
    let program = std::fs::read(executable("PIPELINE")).unwrap();
    let program: frontend::source::Program = serde_json::from_slice(&program).unwrap();
    let entry = frontend::lowering::parse("[Invoke] (Function, [Return] ())")
        .unwrap()
        .rule
        .remove(0)
        .canonical();
    assert_eq!(
        program
            .rule
            .iter()
            .filter(|rule| rule.canonical() == entry)
            .count(),
        1
    );
}

#[test]
fn manifest() {
    let root = directory();
    let runfile = runfiles::Runfiles::create().unwrap();
    let executable = executable("LOCAL");
    let manifest = root.join("MANIFEST");
    let content = ["BUNDLE", "COMMAND", "LAUNCH"]
        .map(|name| {
            let source = std::env::var(name).unwrap();
            format!(
                "{} {}\n",
                source,
                runfile.rlocation_from(&source, "").unwrap().display()
            )
        })
        .concat();
    std::fs::write(&manifest, content).unwrap();
    let output = Command::new(executable)
        .current_dir(&root)
        .env("RUNFILES_MANIFEST_FILE", &manifest)
        .env_remove("RUNFILES_DIR")
        .args(["--json", "--work", "0"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["closed"], false);
}

fn check(root: &std::path::Path, case: &serde_json::Value) -> std::process::Output {
    let path = root.join("case.json");
    std::fs::write(&path, serde_json::to_vec(case).unwrap()).unwrap();
    Command::new(executable("CHECK"))
        .arg(&path)
        .current_dir(root)
        .env("TEST_UNDECLARED_OUTPUTS_DIR", root)
        .output()
        .unwrap()
}

fn limit() -> serde_json::Value {
    serde_json::json!({"configuration": 128, "record": 10000, "coherence": 8, "occurrence": 32, "scope": 16})
}

fn program(root: &std::path::Path, source: &str) -> PathBuf {
    let program = root.join("program.json");
    std::fs::write(
        &program,
        serde_json::to_vec(&frontend::lowering::parse(source).unwrap()).unwrap(),
    )
    .unwrap();
    program
}

#[test]
fn verification() {
    let root = directory();
    let program = program(&root, "[A] B");
    for (input, target, expect, path, work, success) in [
        ("A", "B", "reached", false, 1000, true),
        ("A", "C", "unreachable", false, 1000, true),
        ("A", "C", "reached", false, 1000, false),
        ("A", "B", "unreachable", false, 1000, false),
        ("A", "B", "reached", false, 0, false),
        ("A", "C", "unreachable", false, 0, false),
        ("A", "B", "reached", true, 1000, true),
        ("A", "C", "unreachable", true, 1000, false),
        ("A, [A] C", "C", "reached", false, 1000, true),
        ("A", "B, [B] C", "reached", false, 1000, false),
        ("[", "B", "reached", false, 1000, false),
    ] {
        let output = check(
            &root,
            &serde_json::json!({
                "program": program,
                "source": input,
                "target": [target],
                "expect": expect,
                "path": path,
                "work": work,
                "limit": limit(),
            }),
        );
        assert_eq!(
            output.status.success(),
            success,
            "{input} => {target} ({expect}, path={path}, work={work}): {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    assert!(root.join("0.json").exists());
}

#[test]
fn matching() {
    let root = directory();
    let program = program(&root, "[A] B");
    for (target, expect, work, success) in [
        (vec!["B", "C"], "reached", 1000, true),
        (vec!["B", "D"], "reached", 1000, false),
        (vec!["D", "E"], "unreachable", 1000, true),
        (vec!["B", "D"], "unreachable", 1000, false),
        (vec!["D", "A"], "reached", 0, false),
        (vec!["D", "E"], "unreachable", 0, false),
        (vec!["B", "["], "reached", 1000, false),
        (vec![], "reached", 1000, false),
        (vec!["B"], "unknown", 1000, false),
    ] {
        let output = check(
            &root,
            &serde_json::json!({
                "program": program,
                "source": "A, [A] C",
                "target": target,
                "expect": expect,
                "path": false,
                "work": work,
                "limit": limit(),
            }),
        );
        assert_eq!(
            output.status.success(),
            success,
            "{expect} {target:?} work={work}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn preservation() {
    let root = directory();
    let program = program(&root, "");
    for (target, expect, success) in [
        ("C", "reached", true),
        ("C", "unreachable", false),
        ("C, [A] C", "reached", false),
        ("C, [A] C", "unreachable", true),
    ] {
        let output = check(
            &root,
            &serde_json::json!({
                "program": program,
                "source": "A, [A] C",
                "target": [target],
                "expect": expect,
                "path": false,
                "work": 1000,
                "limit": limit(),
            }),
        );
        assert_eq!(
            output.status.success(),
            success,
            "{target} {expect}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
