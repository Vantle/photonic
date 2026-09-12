use molten::runtime::{Limit, Runtime};

fn declaration(ordinal: usize) -> std::sync::Arc<molten::program::Scope> {
    std::sync::Arc::new(molten::program::Scope {
        name: ordinal.to_string(),
        rule: vec![std::sync::Arc::new(molten::program::Instruction {
            input: vec![vec![molten::program::Symbol::Atom(ordinal)]],
            ..Default::default()
        })],
    })
}

fn captured(value: molten::program::Symbol, capture: Option<usize>) -> molten::program::Symbol {
    match capture {
        None => value,
        Some(frame) => molten::program::Symbol::Rule(
            std::sync::Arc::new(molten::program::Instruction {
                input: vec![vec![value]],
                ..Default::default()
            }),
            Some(frame),
        ),
    }
}

#[test]
fn conjunction() {
    let program =
        molten::lowering::parse(include_str!("../../../example/conjunction.lava")).unwrap();
    let mut runtime = Runtime::new(program);
    runtime.run(12000, Some(Limit::default()));
    let result = runtime.snapshot();
    assert!(result.closed);
    assert!(result.state.iter().any(|state| {
        state.world.len() == 1
            && state.world[0]
                .particle
                .iter()
                .any(|token| token.label == "False")
    }));
}

fn normalize(
    node: &serde_json::Value,
    symbol: &std::collections::BTreeMap<String, usize>,
    scope: &std::collections::BTreeMap<String, usize>,
) -> molten::state::State {
    use molten::program::Symbol;
    use molten::state::{Frame, State, Token, World};
    let optional = |value: &serde_json::Value| value.as_u64().map(|value| value as usize);
    let particle = |value: &serde_json::Value| {
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|token| Token {
                id: token["id"]
                    .as_u64()
                    .map(|value| value as usize)
                    .unwrap_or_else(|| {
                        token["id"]
                            .as_str()
                            .unwrap()
                            .trim_start_matches('r')
                            .parse()
                            .unwrap()
                    }),
                value: captured(
                    Symbol::Atom(symbol[token["label"].as_str().unwrap()]),
                    optional(&token["capture"]),
                ),
            })
            .collect()
    };
    State {
        world: node["world"]
            .as_array()
            .unwrap()
            .iter()
            .map(|world| World {
                frame: world["frame"].as_u64().unwrap() as usize,
                particle: particle(&world["particle"]),
            })
            .collect(),
        frame: node["frame"]
            .as_array()
            .unwrap()
            .iter()
            .map(|frame| Frame {
                scope: declaration(scope[frame["scope"].as_str().unwrap()]),
                parent: optional(&frame["parent"]),
                lexical: optional(&frame["lexical"]),
                held: particle(&frame["held"]),
            })
            .collect(),
    }
    .canonical()
    .state
}

#[test]
fn reference() {
    use std::collections::{BTreeMap, BTreeSet};
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../example/reference.json")).unwrap();
    for case in fixture.as_array().unwrap() {
        let program = serde_json::from_value(case["program"].clone()).unwrap();
        let limit = &case["limit"];
        let limit = Limit {
            record: Limit::default().record,
            state: limit["state"].as_u64().unwrap() as usize,
            world: limit["world"].as_u64().unwrap() as usize,
            cell: limit["cell"].as_u64().unwrap() as usize,
            frame: limit["frame"].as_u64().unwrap() as usize,
        };
        let mut runtime = Runtime::new(program);
        runtime.run(12000, Some(limit));
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
            for world in node["world"].as_array().unwrap() {
                for token in world["particle"].as_array().unwrap() {
                    symbol.insert(token["label"].as_str().unwrap().to_owned());
                }
            }
            for frame in node["frame"].as_array().unwrap() {
                scope.insert(frame["scope"].as_str().unwrap().to_owned());
                for token in frame["held"].as_array().unwrap() {
                    symbol.insert(token["label"].as_str().unwrap().to_owned());
                }
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
        let states = |value: &serde_json::Value, state: &[molten::state::State]| {
            value["state"]
                .as_array()
                .unwrap()
                .iter()
                .zip(state)
                .map(|(node, state)| (state.clone(), node["status"].as_str().unwrap().to_owned()))
                .collect::<BTreeMap<_, _>>()
        };
        assert_eq!(
            states(&actual, &observed),
            states(case, &expected),
            "configuration/support mismatch: {name}"
        );
        let events = |value: &serde_json::Value, state: &[molten::state::State]| {
            value["event"]
                .as_array()
                .unwrap()
                .iter()
                .map(|event| {
                    (
                        state[event["source"].as_u64().unwrap() as usize].clone(),
                        state[event["target"].as_u64().unwrap() as usize].clone(),
                        event["rule"].as_str().unwrap().to_owned(),
                        event["status"].as_str().unwrap().to_owned(),
                    )
                })
                .collect::<BTreeSet<_>>()
        };
        assert_eq!(
            events(&actual, &observed),
            events(case, &expected),
            "event/support mismatch: {name}"
        );
    }
}

#[test]
fn resume() {
    let program = molten::lowering::parse("Seed.A; [Seed] -> @([A] -> B);").unwrap();
    let mut complete = Runtime::new(program.clone());
    complete.run(12000, None);
    let mut paused = Runtime::new(program);
    paused.run(
        12000,
        Some(Limit {
            state: 1,
            ..Limit::default()
        }),
    );
    assert!(!paused.closed());
    assert!(paused.snapshot().deferred > 0);
    paused.run(12000, Some(Limit::default()));
    assert!(paused.closed());
    let expected = serde_json::to_value(complete.snapshot()).unwrap();
    let actual = serde_json::to_value(paused.snapshot()).unwrap();
    assert_eq!(actual["state"], expected["state"]);
    assert_eq!(actual["event"], expected["event"]);
}

#[test]
fn gate() {
    use molten::matching::{Gate, Slot, Term};
    use molten::program::Symbol;
    let mut gate = Gate::new(vec![
        vec![Term {
            value: Symbol::Atom(0),
        }],
        vec![Term {
            value: Symbol::Atom(1),
        }],
    ]);
    let first = Slot {
        binding: Default::default(),
        world: 0,
        token: vec![0],
        position: 0,
    };
    let second = Slot {
        binding: Default::default(),
        world: 1,
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
    use molten::flow::{self, Binding, Closure, Flow, Place};
    use molten::program::{Instruction, Output, Symbol};
    use molten::state::{Frame, State, Token, World};
    use std::collections::{BTreeMap, BTreeSet};
    let root = Frame {
        scope: declaration(0),
        parent: None,
        lexical: None,
        held: Vec::new(),
    };
    let seed = Token {
        id: 0,
        value: Symbol::Atom(0),
    };
    let witness = State {
        world: Vec::new(),
        frame: vec![
            root.clone(),
            Frame {
                scope: declaration(1),
                parent: Some(0),
                lexical: Some(0),
                held: vec![seed.clone()],
            },
            Frame {
                scope: declaration(2),
                parent: Some(0),
                lexical: Some(1),
                held: vec![seed.clone()],
            },
        ],
    };
    for mapped in [false, true] {
        let state = State {
            world: vec![World {
                frame: 0,
                particle: vec![Token {
                    id: 1,
                    value: Symbol::Structure(
                        1,
                        vec![captured(Symbol::Atom(1), mapped.then_some(1))],
                    ),
                }],
            }],
            frame: if mapped {
                vec![root.clone(), witness.frame[1].clone()]
            } else {
                vec![root.clone()]
            },
        };
        let basis = if mapped {
            Place::Held(1, 0)
        } else {
            Place::World(0, 1)
        };
        let flow = Flow {
            resource: BTreeMap::from([
                (Place::Held(1, 0), BTreeSet::from([basis])),
                (Place::Held(2, 0), BTreeSet::from([basis])),
            ]),
            frame: vec![Some(0), mapped.then_some(1), None],
            context: Vec::new(),
        };
        let rule = Instruction {
            output: vec![Output {
                particle: vec![Symbol::Rule(
                    std::sync::Arc::new(Instruction::default()),
                    None,
                )],
                body: None,
            }],
            ..Instruction::default()
        };
        let binding = Binding {
            value: BTreeMap::new(),
            world: BTreeSet::from([0]),
            footprint: BTreeSet::new(),
            exact: BTreeSet::new(),
            read: BTreeSet::from([basis]),
        };
        let output = flow::apply(
            &state,
            0,
            None,
            &rule,
            &binding,
            Some(Closure {
                state: &witness,
                flow: &flow,
                capture: Some(2),
            }),
        )
        .unwrap()
        .canonical();
        let capture = output.state.world[0]
            .particle
            .iter()
            .find_map(|token| {
                if let Symbol::Rule(_, capture) = &token.value {
                    *capture
                } else {
                    None
                }
            })
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
    use molten::program::Symbol;
    use molten::state::{Frame, State, Token, World};
    let root = Frame {
        scope: declaration(0),
        parent: None,
        lexical: None,
        held: Vec::new(),
    };
    for mask in 0..256usize {
        let identity = (0..4)
            .map(|index| mask / 4usize.pow(index) % 4)
            .collect::<Vec<_>>();
        if identity[0] == identity[1] || identity[2] == identity[3] {
            continue;
        }
        let state = State {
            frame: vec![root.clone()],
            world: (0..2)
                .map(|index| World {
                    frame: 0,
                    particle: identity[index * 2..index * 2 + 2]
                        .iter()
                        .map(|&id| Token {
                            id,
                            value: Symbol::Atom(0),
                        })
                        .collect(),
                })
                .collect(),
        };
        let mut changed = state.clone();
        changed.world.reverse();
        for world in &mut changed.world {
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
    let program = molten::lowering::parse("A.A; [A] -> B;").unwrap();
    let execute = || {
        let mut runtime = Runtime::new(program.clone());
        runtime.run(12_000, None);
        assert!(runtime.closed());
        serde_json::to_value(runtime.snapshot()).unwrap()
    };
    let expected = execute();
    for _ in 0..64 {
        assert_eq!(execute(), expected);
    }
}

#[test]
fn inheritance() {
    use molten::flow::{self, Binding, Closure, Flow, Place};
    use molten::program::{Instruction, Output, Symbol};
    use molten::state::{Frame, State, Token, World};
    use std::collections::{BTreeMap, BTreeSet};
    use std::sync::Arc;

    for held in [false, true] {
        for transformed in [false, true] {
            for fragment in [false, true] {
                let value = if fragment {
                    captured(Symbol::Atom(0), Some(0))
                } else {
                    Symbol::Atom(0)
                };
                let original = Token { id: 5, value };
                let root = Frame {
                    scope: declaration(0),
                    parent: None,
                    lexical: None,
                    held: if held {
                        vec![original.clone()]
                    } else {
                        Vec::new()
                    },
                };
                let source = State {
                    world: vec![World {
                        frame: 0,
                        particle: vec![original.clone()],
                    }],
                    frame: vec![root.clone()],
                };
                let witness = State {
                    world: Vec::new(),
                    frame: vec![
                        root,
                        Frame {
                            scope: declaration(1),
                            parent: Some(0),
                            lexical: Some(0),
                            held: vec![Token {
                                id: 9,
                                value: if transformed {
                                    Symbol::Atom(1)
                                } else {
                                    original.value
                                },
                            }],
                        },
                    ],
                };
                let basis = if held {
                    Place::Held(0, 5)
                } else {
                    Place::World(0, 5)
                };
                let mut flow = Flow::identity(&source);
                flow.context.clear();
                flow.frame.push(None);
                flow.resource
                    .insert(Place::Held(1, 9), BTreeSet::from([basis]));
                let rule = Instruction {
                    output: vec![Output {
                        particle: vec![Symbol::Rule(Arc::new(Instruction::default()), None)],
                        body: None,
                    }],
                    ..Instruction::default()
                };
                let binding = Binding {
                    world: BTreeSet::new(),
                    footprint: BTreeSet::new(),
                    exact: BTreeSet::new(),
                    read: BTreeSet::from([basis]),
                    value: BTreeMap::new(),
                };
                let result = flow::apply(
                    &source,
                    0,
                    None,
                    &rule,
                    &binding,
                    Some(Closure {
                        state: &witness,
                        flow: &flow,
                        capture: Some(1),
                    }),
                )
                .unwrap();
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
