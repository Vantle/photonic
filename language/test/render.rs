use crate::runtime::Limit;
use crate::snapshot::{Node, Value};
use std::sync::Arc;

fn verify(state: &[Node]) {
    let text = |node: &Node| {
        node.world
            .iter()
            .flat_map(|world| &world.particle)
            .find_map(|token| match &token.value {
                Value::Atom(atom) if atom.as_ref() == "X" => Some(atom.clone()),
                _ => None,
            })
            .unwrap()
    };
    let first = text(&state[0]);
    for node in state {
        assert!(Arc::ptr_eq(&first, &text(node)));
        assert!(Arc::ptr_eq(&state[0].frame[0].scope, &node.frame[0].scope));
    }
}

#[test]
fn ownership() {
    let report = {
        let program = frontend::lowering::parse("A.X, [A] B, [B] C").unwrap();
        let target = crate::test::target(&program, "C.X");
        let mut search = crate::path::Search::new(program, Some(target));
        search.run(100_000, Limit::default());
        search.report()
    };
    assert_eq!(report.outcome, crate::prism::Outcome::Reached);
    assert_eq!(report.state.len(), 3);
    verify(&report.state);
    let expected = serde_json::to_vec(&report).unwrap();
    assert_eq!(
        std::thread::spawn(move || serde_json::to_vec(&report).unwrap())
            .join()
            .unwrap(),
        expected
    );
    let snapshot = {
        let mut runtime =
            crate::runtime::Runtime::new(&frontend::lowering::parse("A.X, [A] B, [B] C").unwrap());
        runtime.run(100_000, Limit::default());
        runtime.snapshot()
    };
    assert!(snapshot.closed);
    verify(&snapshot.state);
    assert_eq!(snapshot.state.len(), 3);
}

#[test]
fn isolation() {
    let first = crate::program::Program::new(&frontend::lowering::parse("A").unwrap());
    let second = crate::program::Program::new(&frontend::lowering::parse("B").unwrap());
    let render = |program: &crate::program::Program| {
        super::Builder::new(program).node(
            0,
            &crate::state::State::initial(program),
            crate::status::Status::Supported,
        )
    };
    let first = render(&first);
    let second = render(&second);
    assert_eq!(crate::test::atom(&first.world[0].particle[0]), Some("A"));
    assert_eq!(crate::test::atom(&second.world[0].particle[0]), Some("B"));
}

#[test]
fn value() {
    let snapshot = {
        let mut runtime = crate::runtime::Runtime::new(
            &frontend::lowering::parse("⟨x⟩.Seed, [Seed] ().([A] B)").unwrap(),
        );
        runtime.run(100_000, Limit::default());
        runtime.snapshot()
    };
    let token = snapshot
        .state
        .iter()
        .flat_map(|node| &node.world)
        .flat_map(|world| &world.particle)
        .collect::<Vec<_>>();
    let atom = token
        .iter()
        .find(|token| token.value == Value::Atom("⟨x⟩".into()))
        .unwrap();
    assert_eq!(
        serde_json::to_value(atom).unwrap(),
        serde_json::json!({"id": atom.id, "atom": "⟨x⟩"})
    );
    let rule = token
        .iter()
        .find(|token| matches!(token.value, Value::Rule(_)))
        .unwrap();
    let Value::Rule(index) = rule.value else {
        unreachable!("the token is a rule")
    };
    assert_eq!(snapshot.definition[index].name, "[A] B");
    assert_eq!(
        frontend::text::definition(&snapshot.definition[index].rule),
        "[A] B"
    );
    assert_eq!(
        serde_json::to_value(rule).unwrap()["rule"],
        serde_json::json!(index)
    );
}
