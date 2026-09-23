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
    let program: photonic::source::Program =
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
    let source: photonic::source::Program = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!source.rule.is_empty());
    let value = photonic::source::Program {
        rule: source.rule,
        ..photonic::lowering::parse(
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
            "--json", "--path", "--steps", "100000", "--cells", "128", "--states", "1024",
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
        .args(["--json", "--steps", "0"])
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
    let program: photonic::source::Program = serde_json::from_slice(&program).unwrap();
    let entry = photonic::lowering::parse("[Invoke] (Function [Return] ())")
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
        .args(["--json", "--steps", "0"])
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
    serde_json::json!({"state": 128, "record": 10000, "world": 8, "cell": 32, "frame": 16})
}

fn program(root: &std::path::Path, source: &str) -> PathBuf {
    let program = root.join("program.json");
    std::fs::write(
        &program,
        serde_json::to_vec(&photonic::lowering::parse(source).unwrap()).unwrap(),
    )
    .unwrap();
    program
}

#[test]
fn verification() {
    let root = directory();
    let program = program(&root, "[A] B");
    for (input, target, expect, path, step, success) in [
        ("A", "B", "reached", false, 1000, true),
        ("A", "C", "unreachable", false, 1000, true),
        ("A", "C", "reached", false, 1000, false),
        ("A", "B", "unreachable", false, 1000, false),
        ("A", "B", "reached", false, 0, false),
        ("A", "C", "unreachable", false, 0, false),
        ("A", "B", "reached", true, 1000, true),
        ("A", "C", "unreachable", true, 1000, false),
        ("A [A] C", "C", "reached", false, 1000, true),
        ("A", "B [B] C", "reached", false, 1000, false),
        ("[", "B", "reached", false, 1000, false),
    ] {
        let output = check(
            &root,
            &serde_json::json!({
                "program": program,
                "source": input,
                "target": [target],
                "expect": expect,
                "match": "all",
                "path": path,
                "preserve": true,
                "step": step,
                "limit": limit(),
            }),
        );
        assert_eq!(
            output.status.success(),
            success,
            "{input} => {target} ({expect}, path={path}, step={step}): {} {}",
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
    for (target, mode, expect, step, success) in [
        (vec!["B", "C"], "all", "reached", 1000, true),
        (vec!["B", "D"], "all", "reached", 1000, false),
        (vec!["D", "B"], "any", "reached", 1000, true),
        (vec!["D", "E"], "any", "reached", 1000, false),
        (vec!["B", "D"], "any", "unreachable", 1000, true),
        (vec!["D", "E"], "all", "unreachable", 1000, true),
        (vec!["B", "D"], "all", "unreachable", 1000, false),
        (vec!["B", "C"], "any", "unreachable", 1000, false),
        (vec!["D", "A"], "any", "reached", 0, true),
        (vec!["D", "A"], "all", "reached", 0, false),
        (vec!["D", "E"], "any", "unreachable", 0, false),
        (vec!["D", "E"], "all", "unreachable", 0, false),
        (vec!["B", "["], "any", "reached", 1000, false),
        (vec![], "all", "reached", 1000, false),
        (vec!["B"], "invalid", "reached", 1000, false),
        (vec!["B"], "all", "unknown", 1000, false),
    ] {
        let output = check(
            &root,
            &serde_json::json!({
                "program": program,
                "source": "A [A] C",
                "target": target,
                "match": mode,
                "expect": expect,
                "path": false,
                "preserve": true,
                "step": step,
                "limit": limit(),
            }),
        );
        assert_eq!(
            output.status.success(),
            success,
            "{mode} {expect} {target:?} step={step}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn preservation() {
    let root = directory();
    let program = program(&root, "");
    for (target, preserve, expect, success) in [
        ("C", true, "reached", true),
        ("C", false, "reached", false),
        ("C", false, "unreachable", true),
        ("C [A] C", false, "reached", true),
        ("C [A] C", true, "reached", false),
    ] {
        let output = check(
            &root,
            &serde_json::json!({
                "program": program,
                "source": "A [A] C",
                "target": [target],
                "match": "all",
                "expect": expect,
                "path": false,
                "preserve": preserve,
                "step": 1000,
                "limit": limit(),
            }),
        );
        assert_eq!(
            output.status.success(),
            success,
            "{target} preserve={preserve} {expect}: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
