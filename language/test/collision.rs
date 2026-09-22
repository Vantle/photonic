use super::{Record, Search};
use crate::program::{Program, Symbol};
use crate::state::{Frame, State, Token, World};
use std::sync::Arc;

fn state(edge: &[(usize, usize)]) -> State {
    State {
        world: edge
            .iter()
            .map(|&(left, right)| {
                Arc::new(World {
                    frame: 0,
                    particle: [left, right]
                        .into_iter()
                        .map(|id| Token {
                            id,
                            value: Symbol::Atom(0),
                            capture: None,
                        })
                        .collect(),
                })
            })
            .collect(),
        frame: vec![Arc::new(Frame {
            scope: 0,
            parent: None,
            lexical: None,
            particle: Vec::new(),
            held: Vec::new(),
        })]
        .into(),
    }
}

#[test]
fn sharing() {
    let private = state(&[(0, 1), (2, 3)]);
    let shared = state(&[(0, 1), (0, 1)]);
    assert_eq!(
        crate::fingerprint::state(&private),
        crate::fingerprint::state(&shared)
    );
    assert_ne!(
        crate::fingerprint::signature(&private),
        crate::fingerprint::signature(&shared)
    );
}

#[test]
fn permutation() {
    for source in [
        "A.X,B.X [A,B] C",
        "A [A] (B [B] C)",
        "Seed.A [Seed] [A] B",
        "A.X [A] (B [B] C),D [C,D] E",
    ] {
        let mut runtime = crate::runtime::Runtime::new(crate::lowering::parse(source).unwrap());
        runtime.run(100_000, None);
        for state in &runtime.state {
            let expected = crate::fingerprint::signature(state);
            let world = (0..state.world.len()).rev().collect::<Vec<_>>();
            let frame = std::iter::once(0)
                .chain((1..state.frame.len()).rev())
                .collect::<Vec<_>>();
            let mut renamed = state.rename(&world, &frame).state;
            for position in 0..renamed.world.len() {
                let world = Arc::make_mut(&mut renamed.world[position]);
                world.particle.reverse();
                for token in &mut world.particle {
                    token.id += 1000;
                }
            }
            for position in 0..renamed.frame.len() {
                let frame = Arc::make_mut(&mut renamed.frame[position]);
                frame.held.reverse();
                for token in &mut frame.held {
                    token.id += 1000;
                }
            }
            assert_eq!(expected, crate::fingerprint::signature(&renamed));
        }
    }
}

#[test]
fn fallback() {
    let ring = Arc::new(state(&[(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 0)]));
    let triangle = Arc::new(state(&[(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3)]));
    assert_eq!(
        crate::fingerprint::state(&ring),
        crate::fingerprint::state(&triangle)
    );
    assert_eq!(
        crate::fingerprint::signature(&ring),
        crate::fingerprint::signature(&triangle)
    );
    assert_ne!(ring.canonical().state, triangle.canonical().state);
    let source = crate::lowering::parse("A").unwrap();
    let mut search = Search::new(source.clone(), crate::lowering::parse("A").unwrap()).unwrap();
    search.compiled = Arc::new(Program::new(source));
    search.runtime = crate::reduction::Search::new(search.compiled.clone(), ring.clone());
    search.state = vec![Record::new(ring.clone())];
    search.goal = Record::new(triangle);
    search.signature = crate::fingerprint::state(&search.goal.state);
    search.index = std::collections::HashMap::from([(crate::fingerprint::state(&ring), vec![0])]);
    search.initial = true;
    search.reached = false;
    search.run(100_000, crate::runtime::Limit::default());
    assert!(!search.reached);
    assert!(!search.cycle);
    assert_eq!(search.event.len(), 0);
    assert!(search.state[0].canonical.get().is_some());
}

#[test]
fn capture() {
    let mut left = state(&[(0, 1)]);
    left.frame.extend((0..2).map(|_| {
        Arc::new(Frame {
            scope: 1,
            parent: Some(0),
            lexical: Some(0),
            particle: Vec::new(),
            held: Vec::new(),
        })
    }));
    left.world = vec![
        Arc::new(World {
            frame: 1,
            particle: vec![Token {
                id: 0,
                value: Symbol::Atom(0),
                capture: None,
            }],
        }),
        Arc::new(World {
            frame: 2,
            particle: vec![Token {
                id: 1,
                value: Symbol::Atom(1),
                capture: None,
            }],
        }),
        Arc::new(World {
            frame: 0,
            particle: vec![Token {
                id: 2,
                value: Symbol::Rule(0),
                capture: Some(1),
            }],
        }),
    ]
    .into();
    let mut right = left.clone();
    Arc::make_mut(&mut right.world[2]).particle[0].capture = Some(2);
    assert_eq!(
        crate::fingerprint::state(&left),
        crate::fingerprint::state(&right)
    );
    assert_ne!(
        crate::fingerprint::signature(&left),
        crate::fingerprint::signature(&right)
    );
    assert_ne!(left.canonical().state, right.canonical().state);
}

#[test]
fn eviction() {
    let mut search = Search::new(
        crate::lowering::parse("A [A] B [B] C").unwrap(),
        crate::lowering::parse("C").unwrap(),
    )
    .unwrap();
    let graph = state(
        &(0..100)
            .map(|index| (index, (index + 1) % 100))
            .collect::<Vec<_>>(),
    );
    search.structure.advance(&graph);
    assert!(search.structure.retained() > 1000);
    search.run(
        10000,
        crate::runtime::Limit {
            record: 1000,
            ..Default::default()
        },
    );
    assert_eq!(search.summary().outcome, crate::prism::Outcome::Reached);
}

#[test]
fn reporting() {
    for (source, target) in [
        ("A [A] (B [B] (C [C] D))", "D"),
        ("Seed.A [Seed] [A] B", "B.([A] B)"),
        ("A [A] B,C [B,C] D", "D"),
    ] {
        let program = crate::lowering::parse(source).unwrap();
        let target = crate::lowering::parse(target).unwrap();
        let mut actual = Search::new(program.clone(), target.clone()).unwrap();
        let mut expected = Search::new(program, target).unwrap();
        for iteration in 0..1000 {
            let budget = [0, 1, 2, 7, 31][iteration % 5];
            let mut limit = crate::runtime::Limit::default();
            if iteration % 17 == 0 {
                limit.record = 1;
            }
            actual.run(budget, limit);
            expected.run(budget, limit);
            for record in &expected.state {
                record.canonical();
            }
            assert_eq!(
                serde_json::to_value(actual.report()).unwrap(),
                serde_json::to_value(expected.report()).unwrap()
            );
            assert_eq!(
                serde_json::to_value(actual.statistic()).unwrap(),
                serde_json::to_value(expected.statistic()).unwrap()
            );
            if actual.reached {
                break;
            }
        }
        assert!(actual.reached, "{source}");
    }
}
