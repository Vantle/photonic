use super::{Bound, Observation, explore, walk};
use crate::lowering::parse;
use crate::runtime::Limit;
use crate::source::Value;

fn limit() -> Limit {
    Limit {
        state: 4_096,
        record: 10_000_000,
        world: 16,
        cell: 64,
        frame: 10,
    }
}

fn label(observation: &Observation) -> Vec<Vec<String>> {
    let mut result = observation
        .coherence
        .iter()
        .map(|coherence| {
            let mut label = coherence
                .iter()
                .map(|occurrence| match &occurrence.value {
                    Value::Atom(atom) => atom.clone(),
                    Value::Rule { .. } => "rule".to_owned(),
                })
                .collect::<Vec<_>>();
            label.sort();
            label
        })
        .collect::<Vec<_>>();
    result.sort();
    result
}

#[test]
fn reunion() {
    let source = parse("A.X [A] (B) (C) [B, C] D").unwrap();
    let exploration = explore(&source, limit(), Bound::default(), |_| false);
    assert!(exploration.complete());
    assert_eq!(exploration.state, 3);
    assert_eq!(exploration.terminal.len(), 1);
    assert_eq!(label(&exploration.terminal[0]), vec![vec!["D", "X"]]);
}

#[test]
fn production() {
    let source = parse("Seed.A [Seed] [A] B").unwrap();
    let exploration = explore(&source, limit(), Bound::default(), |_| false);
    assert!(exploration.complete());
    assert_eq!(exploration.terminal.len(), 1);
    assert_eq!(label(&exploration.terminal[0]), vec![vec!["B", "rule"]]);
}

#[test]
fn scope() {
    let source = parse(
        "Invoke.Boolean.Not.True
        [Invoke] (Function [Return] ())
        [Function.Boolean.Not.True] Return.False
        [Function.Boolean.Not.False] Return.True",
    )
    .unwrap();
    let exploration = explore(&source, limit(), Bound::default(), |_| false);
    assert!(exploration.complete());
    assert_eq!(exploration.terminal.len(), 1);
    assert_eq!(label(&exploration.terminal[0]), vec![vec!["False"]]);
}

#[test]
fn depth() {
    let source = parse("A, A [A] B [B] C").unwrap();
    let result = walk(&source, limit(), Bound::default(), |_| 0);
    assert_eq!(result.work, 4);
    assert_eq!(result.depth, 2);
    assert_eq!(label(&result.terminal.unwrap()), vec![vec!["C"], vec!["C"]]);
}

#[test]
fn cycle() {
    let source = parse("A [A] B [B] A").unwrap();
    assert!(explore(&source, limit(), Bound::default(), |_| false).cycle);
    assert!(walk(&source, limit(), Bound::default(), |_| 0).cycle);
    let growth = parse("A [A] A.A").unwrap();
    let bounded = Bound {
        state: 64,
        ..Bound::default()
    };
    assert!(!explore(&growth, limit(), bounded, |_| false).complete());
}
