use crate::canonical;
use crate::executor::Executor;
use crate::program::Symbol;
use crate::runtime::{Limit, Runtime};
use crate::search::Search;
use crate::state::{Frame, State, Token, World};
use crate::term::Term;
use std::sync::Arc;
use std::task::Poll;

fn root(world: Vec<World>) -> State {
    State {
        world: world.into_iter().map(Arc::new).collect(),
        frame: vec![
            Frame {
                scope: 0,
                parent: None,
                lexical: None,
                particle: Default::default(),
                held: Vec::new(),
            }
            .into(),
        ]
        .into(),
    }
}

fn token(id: usize) -> Token {
    Token {
        id,
        value: Symbol::Atom(0),
        capture: None,
    }
}

#[test]
fn matching() {
    let state = root(vec![World {
        frame: 0,
        particle: (0..30).map(token).collect(),
    }]);
    let pattern = vec![vec![
        Term {
            value: Symbol::Atom(0),
            capture: None
        };
        15
    ]];
    let mut search = Search::new(
        pattern,
        Arc::new(crate::index::Index::new(Arc::new(state))),
        0,
    );
    let mut found = std::collections::BTreeSet::new();
    for _ in 0..1000 {
        match search.step() {
            Poll::Pending => {}
            Poll::Ready(Some(binding)) => {
                assert_eq!(binding[0].token.len(), 15);
                assert!(found.insert(binding[0].token.clone()));
            }
            Poll::Ready(None) => {
                panic!("155 million combinations cannot be exhausted in 1000 steps")
            }
        }
    }
    assert!(!found.is_empty());
    assert!(!matches!(search.step(), Poll::Ready(None)));
}

#[test]
fn canonicalization() {
    let state = root(
        (0..30)
            .map(|id| World {
                frame: 0,
                particle: vec![token(id), token((id + 1) % 30)],
            })
            .collect(),
    );
    let mut search = canonical::Search::new(Arc::new(state));
    for _ in 0..100 {
        assert!(!search.step());
    }
    assert!(search.finish().is_none());
}

#[test]
fn refinement() {
    let mut state = root(
        (0..12)
            .map(|index| World {
                frame: 0,
                particle: vec![token(index), token(index + 1)],
            })
            .collect(),
    );
    Arc::make_mut(&mut state.world[0]).particle.push(Token {
        id: 20,
        value: Symbol::Atom(1),
        capture: None,
    });
    let mut search = canonical::Search::new(Arc::new(state.clone()));
    let progress = (0..10).find(|_| search.step());
    assert!(
        progress.is_some(),
        "asymmetric sharing should distinguish every coherence"
    );
    let expected = search.finish().unwrap().state;
    state.world.reverse();
    for position in 0..state.world.len() {
        let world = Arc::make_mut(&mut state.world[position]);
        world.particle.reverse();
        for token in &mut world.particle {
            token.id = 100 - token.id;
        }
    }
    assert_eq!(state.canonical().state, expected);
}

#[test]
fn parallel() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("reference.json")).unwrap();
    let executor = [
        Executor::new(1).unwrap(),
        Executor::new(2).unwrap(),
        Executor::new(4).unwrap(),
    ];
    for case in fixture
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["closed"] == true)
    {
        let program: crate::source::Program =
            serde_json::from_value(case["program"].clone()).unwrap();
        let mut runtime = Runtime::new(&program);
        runtime.run(12000, None);
        assert!(runtime.closed(), "{}", case["name"]);
        let expected = serde_json::to_value(runtime.snapshot()).unwrap();
        for executor in &executor {
            let mut runtime = Runtime::new(&program);
            runtime.parallel(executor, 12000, None);
            assert_eq!(
                serde_json::to_value(runtime.snapshot()).unwrap(),
                expected,
                "{}",
                case["name"]
            );
        }
    }
}

#[test]
fn chunking() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!("reference.json")).unwrap();
    for case in fixture
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["closed"] == true)
    {
        let program: crate::source::Program =
            serde_json::from_value(case["program"].clone()).unwrap();
        let mut complete = Runtime::new(&program);
        complete.run(12000, None);
        assert!(complete.closed());
        let expected = serde_json::to_value(complete.snapshot()).unwrap();
        for chunk in [1, 7, 32] {
            let mut runtime = Runtime::new(&program);
            for _ in 0..12000 {
                if runtime.closed() {
                    break;
                }
                runtime.run(chunk, None);
            }
            assert_eq!(
                serde_json::to_value(runtime.snapshot()).unwrap(),
                expected,
                "{}",
                case["name"]
            );
        }
    }
}

#[test]
fn fairness() {
    let source = format!(
        "{},Start [{}] Z [Start] Done",
        vec!["A"; 25].join("."),
        ["A"; 12].join(".")
    );
    let mut runtime = Runtime::new(&crate::lowering::parse(&source).unwrap());
    runtime.run(
        1000,
        Some(Limit {
            cell: 64,
            ..Limit::default()
        }),
    );
    assert!(!runtime.closed());
    assert!(runtime.snapshot().state.iter().any(|node| {
        node.world.iter().any(|world| {
            world
                .particle
                .iter()
                .any(|token| token.label.as_ref() == "Done")
        })
    }));
}

#[test]
fn budget() {
    let source = crate::lowering::parse("Seed.A [Seed] [A] B").unwrap();
    let mut complete = Runtime::new(&source);
    complete.run(12000, None);
    let expected = serde_json::to_value(complete.snapshot()).unwrap();
    for record in [1, complete.record() / 2] {
        let mut runtime = Runtime::new(&source);
        runtime.run(
            12000,
            Some(Limit {
                record,
                ..Limit::default()
            }),
        );
        assert!(!runtime.closed());
        if record == 1 {
            assert_eq!(runtime.snapshot().work, 0);
        } else {
            assert!(runtime.snapshot().work > 0);
        }
        runtime.run(12000, Some(Limit::default()));
        assert_eq!(serde_json::to_value(runtime.snapshot()).unwrap(), expected);
    }
}

#[test]
fn initialization() {
    let source = crate::source::Program {
        initial: vec![vec![crate::source::Value::Atom("A".into())]; 100],
        rule: Vec::new(),
    };
    let runtime = Runtime::new(&source);
    assert_eq!(runtime.snapshot().state[0].world.len(), 100);
    for mask in 0..64 {
        let initial = (0..3)
            .map(|world| {
                (0..2)
                    .filter(|index| mask & (1 << (world * 2 + index)) != 0)
                    .map(|index| crate::source::Value::Atom(index.to_string()))
                    .collect()
            })
            .collect();
        let program = crate::program::Program::new(&crate::source::Program {
            initial,
            rule: Vec::new(),
        });
        let state = State::initial(&program);
        assert_eq!(state, state.canonical().state);
    }
}

#[test]
fn symmetry() {
    let mut state = root(
        (0..30)
            .map(|id| World {
                frame: 0,
                particle: vec![token(id)],
            })
            .collect(),
    );
    let mut search = canonical::Search::new(Arc::new(state.clone()));
    assert!((0..4).any(|_| search.step()));
    let result = search.finish().unwrap();
    assert_eq!(result.state.world.len(), 30);
    assert_eq!(result.resource.len(), 30);
    assert_eq!(result.world.iter().flatten().count(), 30);
    state.world.reverse();
    for position in 0..state.world.len() {
        let world = Arc::make_mut(&mut state.world[position]);
        for token in &mut world.particle {
            token.id += 100;
        }
    }
    assert_eq!(result.state, state.canonical().state);
    let shared = root(vec![
        World {
            frame: 0,
            particle: vec![token(0)]
        };
        30
    ]);
    let mut search = canonical::Search::new(Arc::new(shared));
    assert!((0..4).any(|_| search.step()));
    let shared = search.finish().unwrap();
    assert_eq!(shared.state.world.len(), 30);
    assert_eq!(shared.resource.len(), 1);
    assert_ne!(shared.state, result.state);
}

#[test]
fn identity() {
    let mut state = root(vec![
        World {
            frame: 0,
            particle: vec![token(0), token(1)],
        },
        World {
            frame: 0,
            particle: vec![token(1), token(2)],
        },
    ]);
    Arc::make_mut(&mut state.frame[0]).held.push(token(2));
    let expected = state.canonical();
    for identity in [
        [usize::MAX, 0, usize::MAX / 2],
        [1, usize::MAX, 0],
        [usize::MAX - 1, usize::MAX / 2, usize::MAX],
    ] {
        let mut changed = state.clone();
        for index in 0..changed.world.len() {
            for token in &mut Arc::make_mut(&mut changed.world[index]).particle {
                token.id = identity[token.id];
            }
        }
        for token in &mut Arc::make_mut(&mut changed.frame[0]).held {
            token.id = identity[token.id];
        }
        let actual = changed.canonical();
        assert_eq!(actual.state, expected.state);
        assert_eq!(actual.resource.len(), identity.len());
        for (index, identity) in identity.iter().enumerate() {
            assert_eq!(actual.resource[identity], expected.resource[&index]);
        }
    }
}

#[test]
fn incidence() {
    for encoding in 0..512usize {
        let mut state = root(
            (0..3)
                .map(|world| World {
                    frame: 0,
                    particle: (0..3)
                        .filter(|id| encoding & (1 << (world * 3 + id)) != 0)
                        .map(token)
                        .chain([token(10 + world)])
                        .collect(),
                })
                .collect(),
        );
        state.world.push(state.world[0].clone());
        let expected = state.canonical().state;
        state.world.reverse();
        for position in 0..state.world.len() {
            let world = Arc::make_mut(&mut state.world[position]);
            world.particle.reverse();
            for token in &mut world.particle {
                token.id = 100 - token.id;
            }
        }
        assert_eq!(
            state.canonical().state,
            expected,
            "sharing graph {encoding}"
        );
    }
}

#[test]
fn capture() {
    for count in [1, 2, 4, 16, 65] {
        let closure = Token {
            id: 42,
            value: Symbol::Rule(0),
            capture: Some(1),
        };
        let mut state = root(vec![World {
            frame: 0,
            particle: vec![closure.clone(); count],
        }]);
        state.world.push(state.world[0].clone());
        Arc::make_mut(&mut state.frame[0])
            .held
            .push(closure.clone());
        state.frame.push(Arc::new(Frame {
            scope: 1,
            parent: Some(0),
            lexical: Some(0),
            particle: Default::default(),
            held: vec![closure],
        }));
        state.frame.push(Arc::new(Frame {
            scope: 2,
            parent: None,
            lexical: None,
            particle: Default::default(),
            held: vec![Token {
                id: 99,
                value: Symbol::Rule(1),
                capture: Some(2),
            }],
        }));
        let graph = crate::incidence::Incidence::new(&state);
        assert_eq!(graph.label.len(), 5);
        let capture = graph
            .edge
            .iter()
            .flatten()
            .filter(|&&(kind, _)| kind == crate::link::Link::Capture as u8)
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(
            capture,
            vec![(crate::link::Link::Capture as u8, graph.frame[1])]
        );
        let renamed = state.rename(&[1, 0], &[0, 1]);
        assert_eq!(renamed.resource.len(), 1);
        assert_eq!(renamed.resource[&42], 0);
        assert_eq!(renamed.state.size(), 2 * count + 2);
        for token in renamed
            .state
            .world
            .iter()
            .flat_map(|world| &world.particle)
            .chain(renamed.state.frame.iter().flat_map(|frame| &frame.held))
        {
            assert_eq!(token.id, 0);
            assert_eq!(token.capture, Some(1));
        }
    }
}

#[test]
fn enumeration() {
    let alphabet = [
        Term {
            value: Symbol::Atom(0),
            capture: None,
        },
        Term {
            value: Symbol::Atom(1),
            capture: None,
        },
        Term {
            value: Symbol::Rule(0),
            capture: Some(0),
        },
        Term {
            value: Symbol::Rule(0),
            capture: Some(1),
        },
    ];
    for encoding in 0..64usize {
        let particle = (0..3)
            .map(|id| {
                let term = &alphabet[(encoding >> (id * 2)) & 3];
                Token {
                    id,
                    value: term.value,
                    capture: term.capture,
                }
            })
            .collect::<Vec<_>>();
        for count in 0..=3 {
            for code in 0..4usize.pow(count) {
                let pattern = (0..count)
                    .map(|index| alphabet[(code >> (index * 2)) & 3].clone())
                    .collect::<Vec<_>>();
                let mut expected = std::collections::BTreeSet::new();
                for mask in 0..8usize {
                    if mask.count_ones() != count {
                        continue;
                    }
                    let mut available = particle
                        .iter()
                        .filter(|token| mask & (1 << token.id) != 0)
                        .collect::<Vec<_>>();
                    let matched = pattern.iter().all(|term| {
                        let Some(index) = available.iter().position(|token| {
                            token.value == term.value
                                && (matches!(term.value, Symbol::Atom(_))
                                    || token.capture == term.capture)
                        }) else {
                            return false;
                        };
                        available.remove(index);
                        true
                    });
                    if matched {
                        expected.insert(
                            (0..3)
                                .filter(|id| mask & (1 << id) != 0)
                                .collect::<Vec<_>>(),
                        );
                    }
                }
                let mut state = root(vec![World {
                    frame: 0,
                    particle: particle.clone(),
                }]);
                state.frame.push(state.frame[0].clone());
                let mut search = Search::new(
                    vec![pattern],
                    Arc::new(crate::index::Index::new(Arc::new(state))),
                    0,
                );
                let mut actual = std::collections::BTreeSet::new();
                let mut complete = false;
                for _ in 0..1000 {
                    match search.step() {
                        Poll::Ready(Some(binding)) => {
                            let mut token = binding[0].token.clone();
                            token.sort_unstable();
                            assert!(actual.insert(token));
                        }
                        Poll::Ready(None) => {
                            complete = true;
                            break;
                        }
                        Poll::Pending => {}
                    }
                }
                assert!(complete);
                assert_eq!(
                    actual, expected,
                    "particle {encoding}, pattern {code}, length {count}"
                );
            }
        }
    }
}

#[test]
fn indexing() {
    let alphabet = [
        Term {
            value: Symbol::Atom(0),
            capture: None,
        },
        Term {
            value: Symbol::Atom(1),
            capture: Some(1),
        },
        Term {
            value: Symbol::Rule(0),
            capture: Some(0),
        },
        Term {
            value: Symbol::Rule(0),
            capture: Some(1),
        },
    ];
    for encoding in 0..256usize {
        let mut state = root(
            (0..4)
                .map(|world| World {
                    frame: world % 2,
                    particle: (0..2)
                        .map(|position| {
                            let term = &alphabet[(encoding >> ((world + position) * 2 % 8)) & 3];
                            Token {
                                id: world * 2 + position,
                                value: term.value,
                                capture: term.capture,
                            }
                        })
                        .collect(),
                })
                .collect(),
        );
        state.frame.push(state.frame[0].clone());
        let index = crate::index::Index::new(Arc::new(state.clone()));
        for count in 0..=3 {
            for code in 0..4usize.pow(count) {
                let pattern = (0..count)
                    .map(|position| alphabet[(code >> (position * 2)) & 3].clone())
                    .collect::<Vec<_>>();
                for frame in 0..=2 {
                    let expected = state
                        .world
                        .iter()
                        .enumerate()
                        .filter(|(_, world)| {
                            world.frame == frame
                                && pattern.iter().all(|term| {
                                    world.particle.iter().any(|token| {
                                        token.value == term.value
                                            && (matches!(term.value, Symbol::Atom(_))
                                                || token.capture == term.capture)
                                    })
                                })
                        })
                        .map(|(site, _)| site)
                        .collect::<Vec<_>>();
                    assert_eq!(
                        index
                            .candidate(pattern.iter().cloned(), frame)
                            .into_iter()
                            .map(|site| index.world(site))
                            .collect::<Vec<_>>(),
                        expected
                    );
                }
            }
        }
    }
}

#[test]
fn joining() {
    for equal in [false, true] {
        let pattern = (0..2)
            .map(|position| {
                vec![Term {
                    value: Symbol::Atom(if equal { 0 } else { position }),
                    capture: None,
                }]
            })
            .collect::<Vec<_>>();
        for order in crate::ordering::Ordering::new(0..6, |_| 0) {
            let mut gate = crate::gate::Gate::new(pattern.clone());
            let mut actual = std::collections::BTreeSet::new();
            for index in order {
                let slot = crate::slot::Slot {
                    location: crate::location::Location::World(index / 2),
                    position: index % 2,
                    token: vec![index / 2],
                };
                for binding in gate.arrive(slot) {
                    assert!(
                        actual.insert(
                            binding
                                .iter()
                                .map(|slot| slot.location.world().unwrap())
                                .collect::<Vec<_>>()
                        )
                    );
                }
            }
            let expected = (0..3)
                .flat_map(|left| {
                    (0..3)
                        .filter(move |&right| left != right && (!equal || left < right))
                        .map(move |right| vec![left, right])
                })
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn factorization() {
    for width in 2..=8 {
        let pattern = (0..width)
            .map(|position| {
                vec![Term {
                    value: Symbol::Atom(position),
                    capture: None,
                }]
            })
            .collect::<Vec<_>>();
        for reverse in [false, true] {
            let mut gate = crate::gate::Gate::new(pattern.clone());
            let mut actual = std::collections::BTreeSet::new();
            for arrival in 0..width * 2 {
                let index = if reverse {
                    width * 2 - arrival - 1
                } else {
                    arrival
                };
                let slot = crate::slot::Slot {
                    location: crate::location::Location::World(index),
                    position: index / 2,
                    token: vec![index],
                };
                for binding in gate.arrive(slot) {
                    assert!(
                        actual.insert(
                            binding
                                .iter()
                                .map(|slot| slot.location.world().unwrap())
                                .collect::<Vec<_>>()
                        )
                    );
                }
            }
            let expected = (0..1usize << width)
                .map(|choice| {
                    (0..width)
                        .map(|position| position * 2 + ((choice >> position) & 1))
                        .collect::<Vec<_>>()
                })
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(actual, expected);
        }
    }
}

#[test]
fn bulk() {
    use crate::work::{Result, Work};
    let executor = Executor::new(4).unwrap();
    for width in [16, 4096] {
        let state = Arc::new(root(
            (0..width)
                .map(|position| World {
                    frame: 0,
                    particle: vec![Token {
                        id: position,
                        value: Symbol::Atom(position),
                        capture: None,
                    }],
                })
                .collect(),
        ));
        let expected = state.canonical().state;
        let mut batch = (0..2)
            .map(|position| Work::Normalize(position, canonical::Search::new(state.clone())))
            .collect::<Vec<_>>();
        assert_eq!(Work::parallel(&batch), width == 4096);
        for step in 0..3 {
            let result = if Work::parallel(&batch) {
                executor.map(batch, Work::advance)
            } else {
                batch.into_iter().map(Work::advance).collect()
            };
            batch = Vec::new();
            for (position, result) in result.into_iter().enumerate() {
                let Result::Normalize(index, search, complete) = result else {
                    unreachable!()
                };
                assert_eq!(index, position);
                assert_eq!(complete, step == 2);
                if complete {
                    assert_eq!(search.finish().unwrap().state, expected);
                } else {
                    batch.push(Work::Normalize(index, search));
                }
            }
        }
    }
}

#[test]
fn preparation() {
    use crate::work::{Result, Work};
    let executor = Executor::new(4).unwrap();
    for (width, arity) in [32, 4096]
        .into_iter()
        .flat_map(|width| [1, 8].map(|arity| (width, arity)))
    {
        let state = Arc::new(root(vec![World {
            frame: 0,
            particle: (0..width)
                .map(|id| Token {
                    id,
                    value: Symbol::Atom(id.min(8)),
                    capture: None,
                })
                .collect(),
        }]));
        let index = Arc::new(crate::index::Index::new(state));
        let pattern = vec![
            (0..arity)
                .map(|position| Term::new(Symbol::Atom(position), None))
                .collect::<Vec<_>>(),
        ];
        let mut reference = (0..2)
            .map(|_| crate::search::Search::new(pattern.clone(), index.clone(), 0))
            .collect::<Vec<_>>();
        let mut batch = (0..2)
            .map(|position| {
                Work::Search(
                    position,
                    crate::search::Search::new(pattern.clone(), index.clone(), 0),
                )
            })
            .collect::<Vec<_>>();
        for step in 0..3 {
            assert_eq!(Work::parallel(&batch), width == 4096 && step == 0);
            let result = if Work::parallel(&batch) {
                executor.map(batch, Work::advance)
            } else {
                batch.into_iter().map(Work::advance).collect()
            };
            batch = Vec::new();
            for (position, result) in result.into_iter().enumerate() {
                let Result::Search(identity, search, progress) = result else {
                    unreachable!()
                };
                assert_eq!(position, identity);
                assert_eq!(progress, reference[position].step());
                assert_eq!(search.retained(), reference[position].retained());
                batch.push(Work::Search(identity, search));
            }
        }
    }
}
