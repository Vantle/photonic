use super::{Program, Symbol};

fn program(source: &str) -> Program {
    Program::new(&frontend::lowering::parse(source).unwrap())
}

#[test]
fn identity() {
    for (left, right) in [
        ("[A.B] C", "[B.A] C"),
        ("[A,B] C", "[B,A] C"),
        ("[A] (B, C)", "[A] (C, B)"),
        ("[A] B.([C.D] E)", "[A] B.([D.C] E)"),
        ("[A] (X, [B] C, [C] D)", "[A] (X, [C] D, [B] C)"),
    ] {
        let compiled = program(&format!("{left},\n{right}"));
        assert_eq!(
            compiled.scope[0].rule[0], compiled.scope[0].rule[1],
            "{left}"
        );
    }
    for (left, right) in [
        ("[A] B", "[A] C"),
        ("[A] B", "[A.A] B"),
        ("[A.A] B", "[A,A] B"),
        ("[A] B.C", "[A] (B, C)"),
        ("[A] (X, [B] C)", "[A] (X, [B] D)"),
    ] {
        let compiled = program(&format!("{left},\n{right}"));
        assert_ne!(
            compiled.scope[0].rule[0], compiled.scope[0].rule[1],
            "{left}"
        );
    }
}

#[test]
fn numbering() {
    let compiled = program("().([Seed] Value), [A] (X, [X] Y), [C] D, [A] (X, [X] Y)");
    assert_eq!(compiled.scope[0].rule, [0, 2, 0]);
    assert_eq!(compiled.initial, [[Symbol::Rule(3)]]);
    assert_eq!(
        compiled
            .rule
            .iter()
            .map(|rule| rule.name.as_str())
            .collect::<Vec<_>>(),
        ["[A] (X, [X] Y)", "[X] Y", "[C] D", "[Seed] Value"]
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
    let compiled = program("A.B, [A] (X, [X] Y), [B] C");
    let atom = compiled.atom.len();
    let rule = compiled.rule.len();
    let known =
        compiled.target(&frontend::lowering::parse("B.A, ().([A] (X, [X] Y)), [B] C").unwrap());
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
    let unknown = compiled.target(&frontend::lowering::parse("Z, [B] C, [Q] R").unwrap());
    assert_eq!(unknown.rule, [compiled.scope[0].rule[1], rule]);
    assert_eq!(unknown.initial, [[Symbol::Atom(atom + 2)]]);
    assert_eq!(compiled.atom.len(), atom);
    assert_eq!(compiled.rule.len(), rule);
}

fn structure(compiled: &Program) -> impl PartialEq + std::fmt::Debug + use<> {
    (
        compiled.atom.iter().cloned().collect::<Vec<_>>(),
        compiled
            .rule
            .iter()
            .map(|rule| {
                (
                    rule.input.clone(),
                    rule.output
                        .iter()
                        .map(|output| {
                            (
                                output.particle.clone(),
                                output.body.map(|scope| compiled.scope[scope].rule.clone()),
                            )
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>(),
        compiled.scope[0].rule.clone(),
        compiled.initial.clone(),
    )
}

#[test]
fn meaning() {
    for (term, spelled) in [
        ("[A] [B]", "[A] B, [B] A"),
        ("[A] [B] C", "[A] B, [A] C, [B] A, [B] C"),
        ("C [B] [A]", "[B] A, [B] C, [A] B, [A] C"),
        ("[A, B] [C] D", "[A, B] C, [A, B] D, [C] (A, B), [C] D"),
        (
            "[A] [B] (K, [K] C)",
            "[A] B, [A] (K, [K] C), [B] A, [B] (K, [K] C)",
        ),
        ("[[X] Y] [B]", "[[X] Y] B, [B] ().([X] Y)"),
        ("X.([A] [B] C)", "X.([A] B).([A] C).([B] A).([B] C)"),
        ("[S] (K, [A] [B] C)", "[S] (K, [A] B, [A] C, [B] A, [B] C)"),
    ] {
        assert_eq!(
            structure(&program(term)),
            structure(&program(spelled)),
            "{term}"
        );
    }
    let compiled = program("[A] B, [A] [B] C");
    assert_eq!(compiled.scope[0].rule, [0, 0, 1, 2, 3]);
    assert_eq!(
        compiled
            .rule
            .iter()
            .map(|rule| rule.name.as_str())
            .collect::<Vec<_>>(),
        ["[A] B", "[A] C", "[B] A", "[B] C"]
    );
}

#[test]
fn breadth() {
    let source = (0..100)
        .map(|index| format!("[A{index}]"))
        .chain(["Z".to_owned()])
        .collect::<Vec<_>>()
        .join(" ");
    let compiled = program(&source);
    assert_eq!(compiled.scope[0].rule.len(), 100 * 100);
    assert_eq!(compiled.rule.len(), 100 * 100);
}
