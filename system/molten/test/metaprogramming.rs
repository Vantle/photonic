use molten::lowering;
use molten::runtime::{Limit, Runtime};
use molten::snapshot::Snapshot;

fn evaluate(source: &str) -> Snapshot {
    let mut runtime = Runtime::new(lowering::parse(source).unwrap());
    runtime.run(
        100_000,
        Some(Limit {
            state: 1000,
            cell: 40,
            frame: 30,
            ..Limit::default()
        }),
    );
    let result = runtime.snapshot();
    assert!(
        result.closed,
        "{source}: {} queued, {} deferred",
        result.queued, result.deferred
    );
    result
}

fn contains(result: &Snapshot, particle: &[&str]) -> bool {
    result.state.iter().any(|state| {
        state.world.len() == 1 && {
            let mut actual = state.world[0]
                .particle
                .iter()
                .map(|token| token.display.as_str())
                .collect::<Vec<_>>();
            let mut expected = particle.to_vec();
            actual.sort();
            expected.sort();
            actual == expected
        }
    })
}

#[test]
fn wrapper() {
    let result = evaluate("Box(A.B); [Box($left.$right)] -> Pair(Left($left).Right($right));");
    assert!(contains(&result, &["Pair(Left(A).Right(B))"]));
    assert!(contains(&result, &["Pair(Left(B).Right(A))"]));
    assert!(!contains(&result, &["A", "B"]));
}

#[test]
fn repetition() {
    assert!(contains(
        &evaluate("Pair(A.A); [Pair($x.$x)] -> Same;"),
        &["Same"]
    ));
    assert!(!contains(
        &evaluate("Pair(A.B); [Pair($x.$x)] -> Same;"),
        &["Same"]
    ));
    assert!(!contains(&evaluate("A; [$x.$x] -> Same;"), &["Same"]));
    assert!(contains(&evaluate("A.A; [$x.$x] -> Same;"), &["Same"]));
}

#[test]
fn coherence() {
    assert!(contains(
        &evaluate("Left(A), Right(A); [Left($x), Right($x)] -> Together($x);"),
        &["Together(A)"]
    ));
    assert!(!contains(
        &evaluate("Left(A), Right(B); [Left($x), Right($x)] -> Together($x);"),
        &["Together(A)"]
    ));
}

#[test]
fn construction() {
    let result = evaluate("Box(@([A] -> Old)).A; [Box(@([$input] -> Old))] -> @([$input] -> New);");
    assert!(result.state.iter().any(|state| {
        state
            .world
            .iter()
            .any(|world| world.particle.iter().any(|token| token.label == "New"))
    }));
    assert!(!result.state.iter().any(|state| {
        state
            .world
            .iter()
            .any(|world| world.particle.iter().any(|token| token.label == "Old"))
    }));
}

#[test]
fn absence() {
    assert!(contains(
        &evaluate("Box(A).Present(B); [Box($x)] unless [Present($x)] -> Missing($x);"),
        &["Missing(A)", "Present(B)"]
    ));
    let result = evaluate("Box(A).Present(A); [Box($x)] unless [Present($x)] -> Missing($x);");
    assert!(
        result
            .event
            .iter()
            .all(|event| event.status != molten::support::Status::Supported)
    );
}

#[test]
fn deduction() {
    let result = evaluate("Seed.Extra; [Seed] -> Box(A); [Box($x)] -> Result($x);");
    assert!(contains(&result, &["Result(A)", "Extra"]));
    let event = result
        .event
        .iter()
        .find(|event| {
            event.source == 0
                && result.state[event.target].world[0]
                    .particle
                    .iter()
                    .any(|token| token.display == "Result(A)")
        })
        .unwrap();
    assert_eq!(event.footprint.len(), 1);
    assert!(event.exact.is_empty());
}

#[test]
fn capture() {
    let mut runtime = Runtime::new(lowering::parse("Enter.Make.Call; [Enter] -> { [A] -> Local; [Make] -> Box(@([Call] -> { A; [Local] -> Done; })); }; [Box($rule)] -> $rule; [A] -> Global;").unwrap());
    runtime.run(12_000, None);
    let result = runtime.snapshot();
    assert!(!result.closed);
    assert!(result.state.iter().any(|state| {
        state
            .world
            .iter()
            .any(|world| world.particle.iter().any(|token| token.label == "Done"))
    }));
    assert!(
        result
            .state
            .iter()
            .flat_map(|state| &state.world)
            .flat_map(|world| &world.particle)
            .any(|token| token.capture.is_some_and(|capture| capture != 0))
    );
}

#[test]
fn fragment() {
    let result = evaluate(
        "Enter.Make.Invoke.Call; [Enter] -> { [Make] -> Box(@([Call] -> Local)); }; [Box($rule)] -> @([Invoke] -> $rule);",
    );
    assert!(result.state.iter().any(|state| {
        state
            .world
            .iter()
            .any(|world| world.particle.iter().any(|token| token.label == "Local"))
    }));
    let report = serde_json::to_value(result).unwrap();
    assert!(report["state"].as_array().unwrap().iter().any(|state| {
        state["world"].as_array().unwrap().iter().any(|world| {
            world["particle"].as_array().unwrap().iter().any(|token| {
                let outer = &token["value"];
                outer["kind"] == "rule"
                    && outer["capture"] == 0
                    && outer["output"][0]["particle"][0]["kind"] == "rule"
                    && outer["output"][0]["particle"][0]["capture"]
                        .as_u64()
                        .is_some_and(|capture| capture > 0)
            })
        })
    }));
}

#[test]
fn unresolved() {
    let result = evaluate("A; [A] -> $unbound;");
    assert_eq!(result.state.len(), 1);
    assert!(result.event.is_empty());
    assert!(contains(
        &evaluate("Seed.A; [Seed] -> @([A] unless [Blocked($query)] -> B);"),
        &["Seed", "B"]
    ));
}

#[test]
fn suspension() {
    let source = format!(
        "Box({}); [Box({})] -> Done;",
        (0..12)
            .map(|index| format!("A{index}"))
            .collect::<Vec<_>>()
            .join("."),
        (0..12)
            .map(|index| format!("$x{index}"))
            .collect::<Vec<_>>()
            .join(".")
    );
    let mut runtime = Runtime::new(lowering::parse(&source).unwrap());
    runtime.run(100, None);
    assert!(!runtime.closed());
    assert!(runtime.record() < 100_000);
}

#[test]
fn anchor() {
    use molten::constraint::Constraint;
    use molten::program::{Instruction, Scope, Symbol};
    use molten::state::{Frame, State};
    use std::collections::BTreeMap;
    use std::sync::Arc;
    let root = Frame {
        scope: Arc::new(Scope::default()),
        parent: None,
        lexical: None,
        held: Vec::new(),
    };
    let child = Frame {
        parent: Some(0),
        lexical: Some(0),
        ..root.clone()
    };
    let state = State {
        world: Vec::new(),
        frame: vec![root, child.clone(), child],
    };
    let value = |frame| {
        BTreeMap::from([(
            "rule".into(),
            Symbol::Rule(Arc::new(Instruction::default()), Some(frame)),
        )])
    };
    let constraint = Constraint::new(&state, value(1), &[Some(0), Some(1), Some(2)]);
    assert!(constraint.accepts(&state, &value(1)));
    assert!(!constraint.accepts(&state, &value(2)));
    let mut state = state;
    state.frame.push(Frame {
        parent: Some(1),
        lexical: Some(0),
        ..state.frame[1].clone()
    });
    state.frame.push(Frame {
        parent: Some(2),
        lexical: Some(0),
        ..state.frame[2].clone()
    });
    let constraint = Constraint::new(&state, value(3), &[Some(0), Some(1), Some(2), None, None]);
    assert!(constraint.accepts(&state, &value(3)));
    assert!(!constraint.accepts(&state, &value(4)));
}

#[test]
fn sharing() {
    let result = evaluate(
        "Left.Make, Right.Make; [Left] -> { [Make] -> Box(@([Call] -> X)); }; [Right] -> { [Make] -> Box(@([Call] -> X)); }; [Box($left), Box($right)] -> Pair($left.$right); [Box($same), Box($same)] -> Same;",
    );
    assert!(!contains(&result, &["Same"]));
    let report = serde_json::to_value(result).unwrap();
    assert!(report["state"].as_array().unwrap().iter().any(|state| {
        state["world"].as_array().unwrap().iter().any(|world| {
            world["particle"].as_array().unwrap().iter().any(|token| {
                let value = &token["value"];
                value["name"] == "Pair"
                    && value["particle"][0]["capture"] != value["particle"][1]["capture"]
            })
        })
    }));
}

#[test]
fn parallel() {
    use molten::executor::Executor;
    for source in [
        "Box(A.B); [Box($left.$right)] -> Pair(Left($left).Right($right));",
        "Box(@([A] -> Old)).A; [Box(@([$input] -> Old))] -> @([$input] -> New);",
        "Box(A).Present(B); [Box($x)] unless [Present($x)] -> Missing($x);",
        "Enter.Make.Invoke.Call; [Enter] -> { [Make] -> Box(@([Call] -> Local)); }; [Box($rule)] -> @([Invoke] -> $rule);",
    ] {
        let program = lowering::parse(source).unwrap();
        let mut serial = Runtime::new(program.clone());
        serial.run(100_000, None);
        assert!(serial.closed());
        let expected = serde_json::to_value(serial.snapshot()).unwrap();
        for worker in [1, 2, 4] {
            let executor = Executor::new(worker).unwrap();
            let mut runtime = Runtime::new(program.clone());
            for _ in 0..10_000 {
                if runtime.closed() {
                    break;
                }
                runtime.parallel(&executor, 7, None);
            }
            assert_eq!(
                serde_json::to_value(runtime.snapshot()).unwrap(),
                expected,
                "{source}, {worker} workers"
            );
        }
    }
}
