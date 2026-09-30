use std::process::{Command, Output};

fn execute(name: &str, argument: &[&str]) -> Output {
    let binary = std::env::var_os(name).unwrap();
    let binary = runfiles::Runfiles::create()
        .unwrap()
        .rlocation_from(binary, "")
        .unwrap();
    Command::new(binary).args(argument).output().unwrap()
}

fn failure(name: &str, argument: &[&str], message: &str) {
    let output = execute(name, argument);
    let error = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{name} {argument:?} {error}");
    assert!(error.contains(message), "{name} {argument:?} {error}");
}

#[test]
fn budget() {
    failure(
        "INSPECTION",
        &["--budget", "10", "--sample", "1"],
        "did not close within the budget",
    );
    failure(
        "RUNTIME",
        &["--budget", "1", "--sample", "1"],
        "did not close within the budget",
    );
}

#[test]
fn census() {
    failure(
        "FUZZ",
        &["--seed", &u64::MAX.to_string(), "--count", "2"],
        "pass the last seed",
    );
    let empty = execute("FUZZ", &["--count", "0"]);
    assert_eq!(empty.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&empty.stdout).unwrap();
    assert_eq!(report["written"], 0);
}
