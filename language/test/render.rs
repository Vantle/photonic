use crate::snapshot::Node;
use std::sync::Arc;

fn verify(state: &[Node]) {
    let text = |node: &Node| {
        node.world
            .iter()
            .flat_map(|world| &world.particle)
            .find(|token| token.display.as_ref() == "X")
            .map(|token| (token.label.clone(), token.display.clone()))
            .unwrap()
    };
    let (label, _) = text(&state[0]);
    for node in state {
        let (current, display) = text(node);
        assert!(Arc::ptr_eq(&label, &current));
        assert!(Arc::ptr_eq(&current, &display));
        assert!(Arc::ptr_eq(&state[0].frame[0].scope, &node.frame[0].scope));
    }
}

#[test]
fn ownership() {
    let report = {
        let mut search = {
            let program = crate::lowering::parse("A.X [A] B [B] C").unwrap();
            let target = crate::source::Program {
                rule: program.rule.clone(),
                ..crate::lowering::parse("C.X").unwrap()
            };
            crate::path::Search::new(program, target)
        };
        search.run(100_000, crate::runtime::Limit::default());
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
            crate::runtime::Runtime::new(&crate::lowering::parse("A.X [A] B [B] C").unwrap());
        runtime.run(100_000, None);
        runtime.snapshot()
    };
    assert!(snapshot.closed);
    verify(&snapshot.state);
    assert!(!serde_json::to_vec(&snapshot).unwrap().is_empty());
}

#[test]
fn isolation() {
    let first = crate::program::Program::new(&crate::lowering::parse("A").unwrap());
    let second = crate::program::Program::new(&crate::lowering::parse("B").unwrap());
    let render = |program: &crate::program::Program| {
        super::Builder::new(program).node(
            0,
            &crate::state::State::initial(program),
            crate::support::Status::Supported,
        )
    };
    let first = render(&first);
    let second = render(&second);
    assert_eq!(first.world[0].particle[0].label.as_ref(), "A");
    assert_eq!(second.world[0].particle[0].label.as_ref(), "B");
}
