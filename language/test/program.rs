use super::{Program, Symbol};

fn program(source: &str) -> Program {
    Program::new(&crate::lowering::parse(source).unwrap())
}

#[test]
fn identity() {
    for (left, right) in [
        ("[A.B] C", "[B.A] C"),
        ("[A,B] C", "[B,A] C"),
        ("[A] (B) (C)", "[A] (C) (B)"),
        ("[A] B.([C.D] E)", "[A] B.([D.C] E)"),
        ("[A] (X [B] C [C] D)", "[A] (X [C] D [B] C)"),
    ] {
        let compiled = program(&format!("{left}\n{right}"));
        assert_eq!(
            compiled.scope[0].rule[0], compiled.scope[0].rule[1],
            "{left}"
        );
    }
    for (left, right) in [
        ("[A] B", "[A] C"),
        ("[A] B", "[A.A] B"),
        ("[A.A] B", "[A,A] B"),
        ("[A] B.C", "[A] (B) (C)"),
        ("[A] (X [B] C)", "[A] (X [B] D)"),
    ] {
        let compiled = program(&format!("{left}\n{right}"));
        assert_ne!(
            compiled.scope[0].rule[0], compiled.scope[0].rule[1],
            "{left}"
        );
    }
}

#[test]
fn numbering() {
    let compiled = program("([Seed] Value) [A] (X [X] Y) [C] D [A] (X [X] Y)");
    assert_eq!(compiled.scope[0].rule, [0, 2, 0]);
    assert_eq!(compiled.initial, [[Symbol::Rule(3)]]);
    assert_eq!(
        compiled
            .rule
            .iter()
            .map(|rule| rule.name.as_str())
            .collect::<Vec<_>>(),
        ["[A] (X [X] Y)", "[X] Y", "[C] D", "[Seed] Value"]
    );
    let unnamed = Program::new(
        &serde_json::from_str(
            r#"{"rule": [
                {"input": [["A"]], "output": [{"particle": [{"rule": {"input": [["B"]], "output": [{"particle": ["C"]}]}}]}]},
                {"input": [["D"]], "output": [{"body": [{"input": [["E"]], "output": []}]}]}
            ]}"#,
        )
        .unwrap(),
    );
    assert_eq!(
        unnamed
            .rule
            .iter()
            .map(|rule| rule.name.as_str())
            .collect::<Vec<_>>(),
        ["root/0", "Rule 2", "root/1", "root/1/0/0"]
    );
    assert_eq!(
        compiled
            .scope
            .iter()
            .map(|scope| scope.name.as_str())
            .collect::<Vec<_>>(),
        ["root", "root/0/0"]
    );
}

#[test]
fn target() {
    let compiled = program("A.B [A] (X [X] Y) [B] C");
    let atom = compiled.atom.len();
    let rule = compiled.rule.len();
    let known = compiled.target(&crate::lowering::parse("B.A, ([A] (X [X] Y)) [B] C").unwrap());
    assert_eq!(known.rule, [compiled.scope[0].rule[1]]);
    assert_eq!(
        known.initial,
        [
            vec![
                Symbol::Atom(compiled.atom.get_index_of("B").unwrap()),
                Symbol::Atom(compiled.atom.get_index_of("A").unwrap())
            ],
            vec![Symbol::Rule(compiled.scope[0].rule[0])]
        ]
    );
    let unknown = compiled.target(&crate::lowering::parse("Z [B] C [Q] R").unwrap());
    assert_eq!(unknown.rule, [compiled.scope[0].rule[1], rule]);
    assert_eq!(unknown.initial, [[Symbol::Atom(atom + 2)]]);
    assert_eq!(compiled.atom.len(), atom);
    assert_eq!(compiled.rule.len(), rule);
}
