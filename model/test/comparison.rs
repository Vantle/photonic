use model::comparison::{compare, find};
use model::configuration::Configuration;
use model::context::{self, Frame, Reference};
use model::history::{Branch, History};
use model::occurrence::{self, Occurrence};
use model::structure::{Body, Destination, Input, Output, Particle, Rule, Value};
use model::world::{self, World};

fn declaration(identity: u64, atom: &str) -> Rule {
    let Value::Rule(rule) = super::rule(identity, "Secret", Value::Atom(atom.into())) else {
        panic!()
    };
    *rule
}

fn nested(frame: [u64; 3], depth: usize) -> Value {
    let captured = |identity| {
        Value::Rule(Box::new(Rule {
            context: Reference::Captured(context::Identity(identity)),
            input: Input::new(vec![Particle::new(vec![Value::Atom("Call".into())])]),
            output: Output::new(vec![]),
        }))
    };
    Value::Rule(Box::new(Rule {
        context: context::Identity(frame[0]),
        input: Input::new(vec![Particle::new(vec![
            super::rule(frame[1], "X", Value::Atom("A".into())),
            super::rule(frame[2], "X", Value::Atom("A".into())),
        ])]),
        output: Output::new(vec![Destination {
            particle: Particle::new(vec![]),
            body: Some(
                Body::bind(
                    context::Identity(frame[2]),
                    vec![Rule {
                        context: Reference::Local(0),
                        input: Input::new(vec![]),
                        output: Output::new(vec![Destination {
                            particle: Particle::new(vec![captured(frame[1]), captured(frame[2])]),
                            body: Some(Body::nested(
                                Reference::Local(0),
                                vec![Rule {
                                    context: Reference::Local(depth),
                                    input: Input::new(vec![]),
                                    output: Output::new(vec![]),
                                }],
                            )),
                        }]),
                    }],
                )
                .unwrap(),
            ),
        }]),
    }))
}

fn fixture(frame: [u64; 3], world: [u64; 2], resource: [u64; 4]) -> Configuration {
    let occurrence = [
        Value::Atom("A".into()),
        super::rule(frame[1], "Call", Value::Atom("Secret".into())),
        super::rule(frame[1], "Call", Value::Atom("Secret".into())),
        nested(frame, 1),
    ]
    .into_iter()
    .zip(resource)
    .map(|(value, identity)| Occurrence {
        identity: occurrence::Identity(identity),
        value,
        history: History::default(),
    })
    .collect::<Vec<_>>();
    Configuration::new(
        context::Identity(frame[0]),
        vec![
            World {
                identity: world::Identity(world[0]),
                context: context::Identity(frame[0]),
                occurrence: occurrence.clone(),
            },
            World {
                identity: world::Identity(world[1]),
                context: context::Identity(frame[2]),
                occurrence: vec![occurrence[0].clone()],
            },
        ],
        vec![
            Frame {
                identity: context::Identity(frame[0]),
                parent: None,
                lexical: None,
                declaration: vec![],
                held: vec![],
            },
            Frame {
                identity: context::Identity(frame[1]),
                parent: Some(context::Identity(frame[0])),
                lexical: Some(context::Identity(frame[2])),
                declaration: vec![
                    declaration(frame[1], "Left"),
                    declaration(frame[2], "Other"),
                ],
                held: vec![occurrence[0].clone()],
            },
            Frame {
                identity: context::Identity(frame[2]),
                parent: Some(context::Identity(frame[0])),
                lexical: Some(context::Identity(frame[1])),
                declaration: vec![declaration(frame[2], "Right")],
                held: vec![occurrence[1].clone()],
            },
        ],
        History::default(),
    )
    .unwrap()
}

fn rebuild(
    state: &Configuration,
    edit: impl FnOnce(&mut Vec<World>, &mut Vec<Frame>),
) -> Configuration {
    let mut world = state.world().cloned().collect();
    let mut frame = state.frame().cloned().collect();
    edit(&mut world, &mut frame);
    Configuration::new(state.root(), world, frame, state.history().clone()).unwrap()
}

fn different(left: &Configuration, right: &Configuration) {
    assert!(compare(left, right).is_none());
    assert!(compare(right, left).is_none());
}

#[test]
fn renaming() {
    let source = fixture([0, 1, 2], [0, 1], [0, 1, 2, 3]);
    for frame in [[7, 19, 3], [99, 1, 0], [3, 2, 1]] {
        for world in [[8, 3], [1, 0]] {
            let target = fixture(frame, world, [83, 31, 7, 99]);
            let target = rebuild(&target, |world, frame| {
                world.reverse();
                frame.reverse();
                for world in world {
                    world.occurrence.reverse();
                }
                for frame in frame {
                    frame.declaration.reverse();
                    frame.held.reverse();
                }
            });
            let mapping = compare(&source, &target).unwrap();
            assert_eq!(
                mapping.frame(),
                &frame
                    .into_iter()
                    .enumerate()
                    .map(|(source, target)| (
                        context::Identity(source as u64),
                        context::Identity(target)
                    ))
                    .collect()
            );
            assert_eq!(
                mapping.world(),
                &world
                    .into_iter()
                    .enumerate()
                    .map(|(source, target)| (
                        world::Identity(source as u64),
                        world::Identity(target)
                    ))
                    .collect()
            );
            assert_eq!(
                mapping.occurrence(),
                &[83, 31, 7, 99]
                    .into_iter()
                    .enumerate()
                    .map(|(source, target)| (
                        occurrence::Identity(source as u64),
                        occurrence::Identity(target)
                    ))
                    .collect()
            );
            let reverse = compare(&target, &source).unwrap();
            for (source, target) in mapping.frame() {
                assert_eq!(reverse.frame()[target], *source);
            }
            for (source, target) in mapping.world() {
                assert_eq!(reverse.world()[target], *source);
            }
            for (source, target) in mapping.occurrence() {
                assert_eq!(reverse.occurrence()[target], *source);
            }
        }
    }
}

#[test]
fn context() {
    let source = fixture([0, 1, 2], [0, 1], [0, 1, 2, 3]);
    let target = rebuild(&source, |world, _| {
        let Value::Rule(rule) = &mut world[0].occurrence[2].value else {
            panic!()
        };
        rule.context = context::Identity(2);
    });
    different(&source, &target);
    different(
        &source,
        &rebuild(&source, |_, frame| {
            frame[1].parent = Some(context::Identity(2))
        }),
    );
    different(
        &source,
        &rebuild(&source, |_, frame| {
            frame[1].lexical = Some(context::Identity(1))
        }),
    );
    different(
        &source,
        &rebuild(&source, |world, _| world[1].context = context::Identity(1)),
    );
    different(
        &source,
        &rebuild(&source, |_, frame| {
            frame[1].declaration.push(declaration(1, "Left"))
        }),
    );
    different(
        &source,
        &rebuild(&source, |world, _| {
            world[0].occurrence[3].value = nested([0, 1, 2], 0)
        }),
    );
    different(
        &source,
        &rebuild(&source, |world, _| {
            world[0].occurrence[3].value = nested([0, 1, 1], 1);
        }),
    );
    let target = Configuration::new(
        context::Identity(1),
        source.world().cloned().collect(),
        source.frame().cloned().collect(),
        History::default(),
    )
    .unwrap();
    different(&source, &target);
}

fn matrix(mask: [u8; 3]) -> Configuration {
    Configuration::new(
        context::Identity(0),
        (0..2)
            .map(|world| World {
                identity: world::Identity(world),
                context: context::Identity(0),
                occurrence: mask
                    .iter()
                    .enumerate()
                    .filter(|(_, mask)| **mask & (1 << world) != 0)
                    .map(|(identity, _)| Occurrence {
                        identity: occurrence::Identity(identity as u64),
                        value: Value::Atom("A".into()),
                        history: History::default(),
                    })
                    .collect(),
            })
            .collect(),
        vec![Frame {
            identity: context::Identity(0),
            parent: None,
            lexical: None,
            declaration: vec![],
            held: vec![],
        }],
        History::default(),
    )
    .unwrap()
}

#[test]
fn sharing() {
    let mut pattern = Vec::new();
    for first in 1..=3 {
        for second in 1..=3 {
            for third in 1..=3 {
                pattern.push([first, second, third]);
            }
        }
    }
    for &source in &pattern {
        for &target in &pattern {
            let mut expected = source;
            expected.sort();
            let mut direct = target;
            direct.sort();
            let mut swapped = target.map(|value| ((value & 1) << 1) | ((value & 2) >> 1));
            swapped.sort();
            assert_eq!(
                compare(&matrix(source), &matrix(target)).is_some(),
                expected == direct || expected == swapped,
                "{source:?} {target:?}"
            );
        }
    }
    let source = fixture([0, 1, 2], [0, 1], [0, 1, 2, 3]);
    let source = rebuild(&source, |world, _| {
        let shared = world[0].occurrence[1].clone();
        world[1].occurrence.push(shared);
    });
    different(
        &source,
        &rebuild(&source, |world, frame| {
            frame[2].held = vec![world[0].occurrence[2].clone()]
        }),
    );
}

#[test]
fn constraint() {
    let state = rebuild(&matrix([3, 3, 3]), |_, frame| {
        for identity in [1, 2] {
            frame.push(Frame {
                identity: context::Identity(identity),
                parent: Some(context::Identity(0)),
                lexical: Some(context::Identity(0)),
                declaration: vec![],
                held: vec![],
            });
        }
    });
    let mapping = find(&state, &state, |mapping| {
        mapping.frame()[&context::Identity(1)] == context::Identity(2)
            && mapping.world()[&world::Identity(0)] == world::Identity(1)
            && mapping.occurrence()[&occurrence::Identity(0)] == occurrence::Identity(2)
            && mapping.occurrence()[&occurrence::Identity(2)] == occurrence::Identity(0)
    })
    .unwrap();
    assert_eq!(mapping.frame()[&context::Identity(2)], context::Identity(1));
    assert_eq!(mapping.world()[&world::Identity(1)], world::Identity(0));
    assert_eq!(
        mapping.occurrence()[&occurrence::Identity(1)],
        occurrence::Identity(1)
    );
    assert!(find(&state, &state, |_| false).is_none());
    assert!(
        find(&state, &state, |mapping| mapping
            .world()
            .values()
            .all(|&world| world == world::Identity(0)))
        .is_none()
    );
}

#[test]
fn history() {
    let source = matrix([1, 2, 3]);
    let branch = History::default()
        .decide(model::history::Identity(1), Branch(0))
        .unwrap();
    let target = Configuration::new(
        source.root(),
        source.world().cloned().collect(),
        source.frame().cloned().collect(),
        branch.clone(),
    )
    .unwrap();
    different(&source, &target);
    let changed = rebuild(&target, |world, _| world[0].occurrence[0].history = branch);
    different(&target, &changed);
}

#[test]
fn boundary() {
    let source = matrix([1, 2, 3]);
    let scalar = Value::Atom("A".into());
    let rule = |input, output| {
        Value::Rule(Box::new(Rule {
            context: context::Identity(0),
            input,
            output,
        }))
    };
    let value = [
        scalar.clone(),
        rule(Input::new(vec![]), Output::new(vec![])),
        rule(Input::new(vec![Particle::new(vec![])]), Output::new(vec![])),
        rule(
            Input::new(vec![]),
            Output::new(vec![Destination {
                particle: Particle::new(vec![]),
                body: None,
            }]),
        ),
        rule(
            Input::new(vec![]),
            Output::new(vec![Destination {
                particle: Particle::new(vec![]),
                body: Some(Body::new(context::Identity(0), vec![])),
            }]),
        ),
        rule(
            Input::new(vec![Particle::new(vec![scalar.clone()])]),
            Output::new(vec![]),
        ),
        rule(
            Input::new(vec![Particle::new(vec![scalar.clone(), scalar])]),
            Output::new(vec![]),
        ),
    ];
    for (left, original) in value.iter().enumerate() {
        let original = rebuild(&source, |world, _| {
            world[0].occurrence[0].value = original.clone()
        });
        for (right, target) in value.iter().enumerate() {
            let target = rebuild(&source, |world, _| {
                world[0].occurrence[0].value = target.clone()
            });
            assert_eq!(compare(&original, &target).is_some(), left == right);
        }
    }
    let target = rebuild(&source, |_, frame| {
        frame.push(Frame {
            identity: context::Identity(1),
            parent: None,
            lexical: None,
            declaration: vec![],
            held: vec![],
        })
    });
    different(&source, &target);
    let target = rebuild(&source, |world, _| {
        world.push(World {
            identity: world::Identity(2),
            context: context::Identity(0),
            occurrence: vec![],
        })
    });
    different(&source, &target);
}

#[test]
fn empty() {
    let state = |root, frame| {
        Configuration::new(
            context::Identity(root),
            vec![],
            vec![
                Frame {
                    identity: context::Identity(root),
                    parent: None,
                    lexical: None,
                    declaration: vec![],
                    held: vec![],
                },
                Frame {
                    identity: context::Identity(frame),
                    parent: Some(context::Identity(root)),
                    lexical: Some(context::Identity(frame)),
                    declaration: vec![],
                    held: vec![],
                },
            ],
            History::default(),
        )
        .unwrap()
    };
    let source = state(0, 1);
    let target = state(u64::MAX, 0);
    let mapping = compare(&source, &target).unwrap();
    assert_eq!(
        mapping.frame()[&context::Identity(0)],
        context::Identity(u64::MAX)
    );
    assert_eq!(mapping.frame()[&context::Identity(1)], context::Identity(0));
    assert!(mapping.world().is_empty());
    assert!(mapping.occurrence().is_empty());
}

#[test]
fn restoration() {
    for shared in [false, true] {
        let original = super::archive::initial(shared);
        let value = super::archive::retain(&original, occurrence::Identity(0));
        let mut path = original.clone();
        for _ in 0..3 {
            let code = path
                .target()
                .world()
                .next()
                .unwrap()
                .occurrence
                .iter()
                .find(|value| matches!(value.value, Value::Rule(_)))
                .unwrap()
                .identity;
            let erased = super::archive::remove(&path, vec![code]);
            path = super::archive::restore(&erased, value.clone(), 0, occurrence::Identity(0));
            let mapping = compare(original.source(), path.target()).unwrap();
            for (&source, &target) in mapping.frame() {
                assert_eq!(path.flow().frame[&target], Some(source));
                let frame = original
                    .source()
                    .frame()
                    .find(|frame| frame.identity == source)
                    .unwrap();
                for value in &frame.held {
                    assert_eq!(
                        path.flow().resource[&model::flow::Place::Held(
                            target,
                            mapping.occurrence()[&value.identity]
                        )],
                        std::collections::BTreeSet::from([model::flow::Place::Held(
                            source,
                            value.identity
                        )])
                        .union(&if shared {
                            std::collections::BTreeSet::from([model::flow::Place::World(
                                world::Identity(0),
                                value.identity,
                            )])
                        } else {
                            std::collections::BTreeSet::new()
                        })
                        .copied()
                        .collect()
                    );
                }
            }
            for (&source, &target) in mapping.world() {
                assert_eq!(
                    path.flow().context[&target],
                    std::collections::BTreeSet::from([source])
                );
                let world = original
                    .source()
                    .world()
                    .find(|world| world.identity == source)
                    .unwrap();
                for value in &world.occurrence {
                    assert_eq!(
                        path.flow().resource[&model::flow::Place::World(
                            target,
                            mapping.occurrence()[&value.identity]
                        )],
                        std::collections::BTreeSet::from([model::flow::Place::World(
                            source,
                            value.identity
                        )])
                    );
                }
            }
            super::publication::validate(&path);
        }
    }
}
