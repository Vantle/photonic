use crate::runtime::{Limit, Runtime};

#[test]
fn conjunction() {
    let program =
        frontend::lowering::parse(include_str!("../../program/language/conjunction.wave")).unwrap();
    let mut runtime = Runtime::new(&program);
    runtime.run(12000, Limit::default());
    let result = runtime.snapshot();
    assert!(result.closed);
    assert!(result.state.iter().any(|state| {
        state.world.len() == 1
            && state.world[0]
                .particle
                .iter()
                .any(|token| crate::test::atom(token) == Some("False"))
    }));
}

fn normalize(
    node: &serde_json::Value,
    symbol: &std::collections::BTreeMap<String, usize>,
    scope: &std::collections::BTreeMap<String, usize>,
) -> crate::state::State {
    use crate::program::Symbol;
    use crate::state::{Frame, State, Token, World};
    let optional = |value: &serde_json::Value| value.as_u64().map(|value| value as usize);
    let particle = |value: &serde_json::Value| {
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|token| Token {
                id: token["id"].as_u64().map_or_else(
                    || {
                        token["id"]
                            .as_str()
                            .unwrap()
                            .trim_start_matches('r')
                            .parse()
                            .unwrap()
                    },
                    |value| value as usize,
                ),
                value: token["rule"].as_u64().map_or_else(
                    || Symbol::Atom(symbol[token["atom"].as_str().unwrap()]),
                    |rule| Symbol::Rule(rule as usize),
                ),
                capture: optional(&token["capture"]),
            })
            .collect()
    };
    State {
        world: node["world"]
            .as_array()
            .unwrap()
            .iter()
            .map(|world| {
                World {
                    frame: world["frame"].as_u64().unwrap() as usize,
                    particle: particle(&world["particle"]),
                }
                .into()
            })
            .collect(),
        frame: node["frame"]
            .as_array()
            .unwrap()
            .iter()
            .map(|frame| {
                Frame {
                    scope: scope[frame["scope"].as_str().unwrap()],
                    parent: optional(&frame["parent"]),
                    lexical: optional(&frame["lexical"]),
                    particle: Default::default(),
                    held: particle(&frame["held"]),
                }
                .into()
            })
            .collect(),
    }
    .canonical()
    .state
}

#[test]
fn reference() {
    use std::collections::{BTreeMap, BTreeSet};
    let fixture: serde_json::Value = serde_json::from_str(include_str!("reference.json")).unwrap();
    for case in fixture.as_array().unwrap() {
        let program = serde_json::from_value(case["program"].clone()).unwrap();
        let limit = &case["limit"];
        let limit = Limit {
            record: 1_000_000,
            configuration: limit["state"].as_u64().unwrap() as usize,
            coherence: limit["world"].as_u64().unwrap() as usize,
            occurrence: limit["cell"].as_u64().unwrap() as usize
                + if case["closed"] == true { 64 } else { 0 },
            scope: limit["frame"].as_u64().unwrap() as usize,
        };
        let mut runtime = Runtime::new(&program);
        runtime.run(12000, limit);
        let actual = serde_json::to_value(runtime.snapshot()).unwrap();
        let name = case["name"].as_str().unwrap();
        if !case["closed"].as_bool().unwrap() {
            assert!(!runtime.closed(), "{name} must remain suspended");
            continue;
        }
        assert!(
            runtime.closed(),
            "{name} did not close: {}",
            actual["queued"]
        );
        let mut symbol = BTreeSet::new();
        let mut scope = BTreeSet::new();
        for node in case["state"]
            .as_array()
            .unwrap()
            .iter()
            .chain(actual["state"].as_array().unwrap())
        {
            let world = node["world"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|world| world["particle"].as_array().unwrap());
            let held = node["frame"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|frame| frame["held"].as_array().unwrap());
            symbol.extend(
                world
                    .chain(held)
                    .filter_map(|token| token["atom"].as_str())
                    .map(str::to_owned),
            );
            for frame in node["frame"].as_array().unwrap() {
                scope.insert(frame["scope"].as_str().unwrap().to_owned());
            }
        }
        let symbol = symbol
            .into_iter()
            .enumerate()
            .map(|(index, value)| (value, index))
            .collect::<BTreeMap<_, _>>();
        let scope = scope
            .into_iter()
            .enumerate()
            .map(|(index, value)| (value, index))
            .collect::<BTreeMap<_, _>>();
        let expected = case["state"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| normalize(node, &symbol, &scope))
            .collect::<Vec<_>>();
        let observed = actual["state"]
            .as_array()
            .unwrap()
            .iter()
            .map(|node| normalize(node, &symbol, &scope))
            .collect::<Vec<_>>();
        let state = |value: &serde_json::Value, state: &[crate::state::State]| {
            value["state"]
                .as_array()
                .unwrap()
                .iter()
                .zip(state)
                .map(|(node, state)| (state.clone(), node["status"].as_str().unwrap().to_owned()))
                .collect::<BTreeMap<_, _>>()
        };
        assert_eq!(
            state(&actual, &observed),
            state(case, &expected),
            "configuration/support mismatch: {name}"
        );
        let event = |value: &serde_json::Value, state: &[crate::state::State]| {
            value["event"]
                .as_array()
                .unwrap()
                .iter()
                .map(|event| {
                    (
                        state[event["source"].as_u64().unwrap() as usize].clone(),
                        state[event["target"].as_u64().unwrap() as usize].clone(),
                        event["rule"].as_str().map_or_else(
                            || {
                                value["definition"][event["rule"].as_u64().unwrap() as usize]
                                    ["name"]
                                    .as_str()
                                    .unwrap()
                                    .to_owned()
                            },
                            str::to_owned,
                        ),
                        event["status"].as_str().unwrap().to_owned(),
                    )
                })
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(
            event(&actual, &observed),
            event(case, &expected),
            "event/support mismatch: {name}"
        );
    }
}

#[test]
fn resume() {
    let program = frontend::lowering::parse("Seed.A, [Seed] ().([A] B)").unwrap();
    let mut complete = Runtime::new(&program);
    complete.run(12000, Limit::default());
    let mut paused = Runtime::new(&program);
    paused.run(
        12000,
        Limit {
            configuration: 1,
            ..Limit::default()
        },
    );
    assert!(!paused.closed());
    assert!(paused.snapshot().deferred > 0);
    paused.run(12000, Limit::default());
    assert!(paused.closed());
    let expected = serde_json::to_value(complete.snapshot()).unwrap();
    let actual = serde_json::to_value(paused.snapshot()).unwrap();
    assert_eq!(actual["state"], expected["state"]);
    assert_eq!(actual["event"], expected["event"]);
}

#[test]
fn slice() {
    for source in [
        "A.A, [A] B, [A] C, [B, C] D",
        "Seed.A, [Seed] ().([A] B)",
        "A, [A] (B, [B] C), [C] D",
    ] {
        let program = frontend::lowering::parse(source).unwrap();
        let mut whole = Runtime::new(&program);
        whole.run(100_000, Limit::default());
        let mut sliced = Runtime::new(&program);
        while !sliced.closed() {
            sliced.run(1, Limit::default());
        }
        let expected = serde_json::to_value(whole.snapshot()).unwrap();
        let actual = serde_json::to_value(sliced.snapshot()).unwrap();
        for field in ["work", "state", "event"] {
            assert_eq!(actual[field], expected[field], "{field}: {source}");
        }
    }
}

#[test]
fn gate() {
    use crate::gate::Gate;
    use crate::program::Symbol;
    use crate::slot::Slot;
    use crate::term::Term;
    let mut gate = Gate::new(vec![
        vec![Term {
            value: Symbol::Atom(0),
            capture: None,
        }],
        vec![Term {
            value: Symbol::Atom(1),
            capture: None,
        }],
    ]);
    let first = Slot {
        location: crate::location::Location::World(0),
        token: vec![0],
        position: 0,
    };
    let second = Slot {
        location: crate::location::Location::World(1),
        token: vec![1],
        position: 1,
    };
    assert!(gate.arrive(second.clone()).is_empty());
    assert!(gate.arrive(second).is_empty());
    assert_eq!(gate.arrive(first.clone()).len(), 1);
    assert!(gate.arrive(first).is_empty());
}

#[test]
fn capture() {
    use crate::flow::{Binding, Closure, Flow};
    use crate::place::Place;
    use crate::program::{Instruction, Output, Symbol};
    use crate::state::{Frame, State, Token, World};
    use std::collections::BTreeSet;
    let root = std::sync::Arc::new(Frame {
        scope: 0,
        parent: None,
        lexical: None,
        particle: Default::default(),
        held: Vec::new(),
    });
    let seed = Token {
        id: 0,
        value: Symbol::Atom(0),
        capture: None,
    };
    let witness = State {
        world: Vec::new().into(),
        frame: vec![
            root.clone(),
            Frame {
                scope: 1,
                parent: Some(0),
                lexical: Some(0),
                particle: Default::default(),
                held: vec![seed.clone()],
            }
            .into(),
            Frame {
                scope: 2,
                parent: Some(0),
                lexical: Some(1),
                particle: Default::default(),
                held: vec![seed],
            }
            .into(),
        ]
        .into(),
    };
    for mapped in [false, true] {
        let state = State {
            world: vec![
                World {
                    frame: 0,
                    particle: vec![Token {
                        id: 1,
                        value: Symbol::Atom(1),
                        capture: mapped.then_some(1),
                    }],
                }
                .into(),
            ]
            .into(),
            frame: if mapped {
                vec![root.clone(), witness.frame[1].clone()].into()
            } else {
                vec![root.clone()].into()
            },
        };
        let basis = if mapped {
            Place::Held(1, 0)
        } else {
            Place::World(0, 1)
        };
        let flow = Flow {
            resource: [
                (Place::Held(1, 0), crate::basis::Set::single(basis)),
                (Place::Held(2, 0), crate::basis::Set::single(basis)),
            ]
            .into_iter()
            .collect(),
            frame: vec![Some(0), mapped.then_some(1), None],
            context: Vec::new(),
        };
        let rule = Instruction {
            output: vec![Output {
                particle: vec![Symbol::Rule(0)],
                body: None,
            }],
            ..Instruction::default()
        };
        let binding = Binding {
            world: BTreeSet::from([0]).into(),
            footprint: BTreeSet::new().into(),
            exact: BTreeSet::new().into(),
            read: BTreeSet::from([basis]).into(),
        };
        let output = crate::application::apply(crate::application::Request {
            scope: &[],
            source: &state,
            frame: 0,
            owner: crate::application::Owner::Capture(Closure {
                state: &witness,
                flow: &flow,
                capture: 2,
            }),
            rule: &rule,
            binding: &binding,
        })
        .canonical();
        let capture = output.state.world[0]
            .particle
            .iter()
            .find(|token| matches!(token.value, Symbol::Rule(_)))
            .unwrap()
            .capture
            .unwrap();
        assert_eq!(output.state.environment(capture), witness.environment(2));
        assert_eq!(
            output
                .state
                .frame
                .iter()
                .flat_map(|frame| frame.held.iter().map(|token| token.id))
                .collect::<BTreeSet<_>>()
                .len(),
            1
        );
    }
}

#[test]
fn permutation() {
    use crate::program::Symbol;
    use crate::state::{Frame, State, Token, World};
    let root = std::sync::Arc::new(Frame {
        scope: 0,
        parent: None,
        lexical: None,
        particle: Default::default(),
        held: Vec::new(),
    });
    for mask in 0..256usize {
        let identity = (0..4)
            .map(|index| mask / 4usize.pow(index) % 4)
            .collect::<Vec<_>>();
        if identity[0] == identity[1] || identity[2] == identity[3] {
            continue;
        }
        let state = State {
            frame: vec![root.clone()].into(),
            world: (0..2)
                .map(|index| {
                    World {
                        frame: 0,
                        particle: identity[index * 2..index * 2 + 2]
                            .iter()
                            .map(|&id| Token {
                                id,
                                value: Symbol::Atom(0),
                                capture: None,
                            })
                            .collect(),
                    }
                    .into()
                })
                .collect(),
        };
        let mut changed = state.clone();
        changed.world.reverse();
        for position in 0..changed.world.len() {
            let world = std::sync::Arc::make_mut(&mut changed.world[position]);
            world.particle.reverse();
            for token in &mut world.particle {
                token.id = 17 - token.id;
            }
        }
        assert_eq!(state.canonical().state, changed.canonical().state);
        assert_eq!(
            state.canonical().state.canonical().state,
            state.canonical().state
        );
    }
}

#[test]
fn determinism() {
    let program = frontend::lowering::parse("A.A, [A] B").unwrap();
    let execute = || {
        let mut runtime = Runtime::new(&program);
        runtime.run(12_000, Limit::default());
        assert!(runtime.closed());
        serde_json::to_value(runtime.snapshot()).unwrap()
    };
    let expected = execute();
    for _ in 0..64 {
        assert_eq!(execute(), expected);
    }
}

#[test]
fn identity() {
    for source in ["X, [X] Y, [W] V, [[W] V] U", "[A] B, [A] B, [[A] B] C"] {
        let program = frontend::lowering::parse(source).unwrap();
        let mut runtime = Runtime::new(&program);
        runtime.run(12_000, Limit::default());
        assert!(runtime.closed(), "{source}");
        let snapshot = runtime.snapshot();
        let mut seen = std::collections::HashSet::new();
        for event in &snapshot.event {
            let binding = (
                event.source,
                &event.rule,
                &event.footprint,
                &event.exact,
                &event.read,
                &event.world,
            );
            assert!(seen.insert(binding), "{source}: {event:?}");
        }
    }
}

#[test]
fn observation() {
    for source in [
        "Seed.A, [Seed] ().([A] B), [B] C",
        "A, [A] (B, C), [B,C] D",
        "A, [A] (B, [B] C)",
    ] {
        let program = frontend::lowering::parse(source).unwrap();
        let mut complete = Runtime::new(&program);
        complete.run(100_000, Limit::default());
        assert!(complete.closed(), "{source}");
        let expected = serde_json::to_value(complete.snapshot()).unwrap();
        let mut observed = Runtime::new(&program);
        for _ in 0..100_000 {
            observed.snapshot();
            if observed.closed() {
                break;
            }
            observed.run(1, Limit::default());
        }
        assert!(observed.closed(), "{source}");
        let actual = serde_json::to_value(observed.snapshot()).unwrap();
        for field in ["state", "event", "view"] {
            assert_eq!(actual[field], expected[field], "{source}: {field}");
        }
    }
}

#[test]
fn inheritance() {
    use crate::flow::{Binding, Closure, Flow};
    use crate::place::Place;
    use crate::program::{Instruction, Output, Symbol};
    use crate::state::{Frame, State, Token, World};
    use std::collections::BTreeSet;

    for held in [false, true] {
        for transformed in [false, true] {
            for fragment in [false, true] {
                let value = if fragment {
                    Symbol::Rule(0)
                } else {
                    Symbol::Atom(0)
                };
                let original = Token {
                    id: 5,
                    value,
                    capture: fragment.then_some(0),
                };
                let root = std::sync::Arc::new(Frame {
                    scope: 0,
                    parent: None,
                    lexical: None,
                    particle: Default::default(),
                    held: if held {
                        vec![original.clone()]
                    } else {
                        Vec::new()
                    },
                });
                let source = State {
                    world: vec![
                        World {
                            frame: 0,
                            particle: vec![original.clone()],
                        }
                        .into(),
                    ]
                    .into(),
                    frame: vec![root.clone()].into(),
                };
                let witness = State {
                    world: Vec::new().into(),
                    frame: vec![
                        root,
                        Frame {
                            scope: 1,
                            parent: Some(0),
                            lexical: Some(0),
                            particle: Default::default(),
                            held: vec![Token {
                                id: 9,
                                value: if transformed {
                                    Symbol::Atom(1)
                                } else {
                                    original.value
                                },
                                capture: (!transformed).then_some(original.capture).flatten(),
                            }],
                        }
                        .into(),
                    ]
                    .into(),
                };
                let basis = if held {
                    Place::Held(0, 5)
                } else {
                    Place::World(0, 5)
                };
                let mut flow = Flow::identity(&source);
                flow.context.clear();
                flow.frame.push(None);
                flow.resource = flow
                    .resource
                    .into_iter()
                    .chain([(Place::Held(1, 9), crate::basis::Set::single(basis))])
                    .collect();
                let rule = Instruction {
                    output: vec![Output {
                        particle: vec![Symbol::Rule(1)],
                        body: None,
                    }],
                    ..Instruction::default()
                };
                let binding = Binding {
                    world: BTreeSet::new().into(),
                    footprint: BTreeSet::new().into(),
                    exact: BTreeSet::new().into(),
                    read: BTreeSet::from([basis]).into(),
                };
                let result = crate::application::apply(crate::application::Request {
                    scope: &[],
                    source: &source,
                    frame: 0,
                    owner: crate::application::Owner::Capture(Closure {
                        state: &witness,
                        flow: &flow,
                        capture: 1,
                    }),
                    rule: &rule,
                    binding: &binding,
                });
                assert_eq!(result.state.frame[1].held[0].id == 5, !transformed);
                let expected = if transformed { 3 } else { 2 };
                let result = result.canonical();
                let identity = result
                    .state
                    .world
                    .iter()
                    .flat_map(|world| &world.particle)
                    .chain(result.state.frame.iter().flat_map(|frame| &frame.held))
                    .map(|token| token.id)
                    .collect::<BTreeSet<_>>();
                assert_eq!(identity.len(), expected);
            }
        }
    }
}
