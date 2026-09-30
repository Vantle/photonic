use super::{Failure, assembled};
use std::path::PathBuf;

fn directory(name: &str) -> PathBuf {
    let path = PathBuf::from(std::env::var_os("TEST_TMPDIR").unwrap()).join(name);
    std::fs::create_dir_all(path.join("program/sample")).unwrap();
    path
}

#[test]
fn empty() {
    let bin = directory("empty");
    assert!(matches!(assembled(&bin), Err(Failure::Empty { .. })));
}

#[test]
fn malformed() {
    let bin = directory("malformed");
    std::fs::write(bin.join("program/sample/broken.program.json"), "{").unwrap();
    let Err(failure) = assembled(&bin) else {
        panic!("a malformed program was read");
    };
    assert!(
        failure.to_string().contains("broken.program.json"),
        "{failure}"
    );
}

#[test]
fn program() {
    let bin = directory("program");
    let program = frontend::lowering::parse("A, [A] B").unwrap();
    std::fs::write(
        bin.join("program/sample/pair.program.json"),
        serde_json::to_string(&program).unwrap(),
    )
    .unwrap();
    let entry = assembled(&bin).unwrap();
    assert_eq!(entry.len(), 1);
    assert_eq!(entry[0].name, "program/sample:pair");
    assert_eq!(entry[0].group, "program");
}
