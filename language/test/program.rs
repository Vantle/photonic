use super::{Output, Program, Symbol};

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
        ("[A] (X, Y, [B] C)", "[A] (Y, X, [B] C)"),
        (
            "[A] ((X, [X] Y), (Z, [Z] W), [B] C)",
            "[A] ([B] C, (Z, [Z] W), (X, [X] Y))",
        ),
        ("[A] ((X, [B] C))", "[A] (X, [B] C)"),
        ("[A] ([B] C)", "[A] ((), [B] C)"),
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
        ("[A] (X, [B] C)", "[A] (X, X, [B] C)"),
        ("[A] (X, [B] C)", "[A] (X, (), [B] C)"),
        ("[A] ((X, [X] Y), [B] C)", "[A] (X, [X] Y, [B] C)"),
        ("[A] ((X, [X] Y), [B] C)", "[A] ((X, [X] W), [B] C)"),
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
    assert_eq!(compiled.scope[0].initial, [[Symbol::Rule(3)]]);
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
                {"input": [["A"]], "output": [[{"rule": {"input": [["B"]], "output": [["C"]]}}]]},
                {"input": [["D"]], "output": [{"initial": [[]], "rule": [{"input": [["E"]], "output": []}]}]}
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
    let known = compiled
        .target(&frontend::lowering::parse("B.A, ().([A] (X, [X] Y)), [B] C").unwrap())
        .unwrap();
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
    let unknown = compiled
        .target(&frontend::lowering::parse("Z, [B] C, [Q] R").unwrap())
        .unwrap();
    assert_eq!(unknown.rule, [compiled.scope[0].rule[1], rule]);
    assert_eq!(unknown.initial, [[Symbol::Atom(atom + 2)]]);
    assert_eq!(compiled.atom.len(), atom);
    assert_eq!(compiled.rule.len(), rule);
    assert!(
        compiled
            .target(&frontend::lowering::parse("A.B, (X, [X] Y)").unwrap())
            .is_none()
    );
}

#[test]
fn scope() {
    let compiled = program("Z, (X, [X] Y), [A] ((B, [B] C), D, [D] E)");
    assert_eq!(
        compiled
            .scope
            .iter()
            .map(|scope| (scope.name.as_str(), scope.opener))
            .collect::<Vec<_>>(),
        [
            ("root", None),
            ("root/0/0", Some(0)),
            ("root/0/0/1", Some(0)),
            ("root/1", None),
        ]
    );
    assert_eq!(compiled.scope[0].scope, [3]);
    assert_eq!(compiled.scope[1].scope, [2]);
    assert_eq!(compiled.scope[1].rule, [1]);
    assert_eq!(compiled.scope[2].rule, [2]);
    assert_eq!(compiled.scope[3].rule, [3]);
    let atom = |name: &str| Symbol::Atom(compiled.atom.get_index_of(name).unwrap());
    assert_eq!(compiled.scope[0].initial, [[atom("Z")]]);
    assert_eq!(compiled.scope[1].initial, [[atom("D")]]);
    assert_eq!(compiled.scope[2].initial, [[atom("B")]]);
    assert_eq!(compiled.scope[3].initial, [[atom("X")]]);
    assert!(matches!(compiled.rule[0].output[..], [Output::Scope(1)]));
    let repeated = program("(X, [X] Y), (X, [X] Y), [A] ((B, [B] C), (B, [B] C))");
    assert_eq!(repeated.scope.len(), 3);
    assert_eq!(repeated.scope[0].scope, [2, 2]);
    assert!(matches!(
        repeated.rule[0].output[..],
        [Output::Scope(1), Output::Scope(1)]
    ));
    let opened = program("[A] (B, [B] C), [D] (B, [B] C)");
    assert_eq!(opened.scope.len(), 3);
    assert_eq!(
        opened
            .scope
            .iter()
            .map(|scope| scope.opener)
            .collect::<Vec<_>>(),
        [None, Some(0), Some(2)]
    );
    assert_eq!(
        compiled.definition(0),
        frontend::lowering::parse("[A] ((B, [B] C), D, [D] E)")
            .unwrap()
            .rule
            .remove(0)
            .canonical()
    );
}

fn outline(compiled: &Program, scope: usize) -> String {
    let scope = &compiled.scope[scope];
    let nested = scope
        .scope
        .iter()
        .map(|&scope| outline(compiled, scope))
        .collect::<Vec<_>>();
    format!("({:?}, {:?}, {nested:?})", scope.initial, scope.rule)
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
                        .map(|output| match output {
                            Output::Particle(particle) => format!("{particle:?}"),
                            Output::Scope(scope) => outline(compiled, *scope),
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>(),
        outline(compiled, 0),
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
