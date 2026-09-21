use model::configuration::Configuration;
use model::context::{self, Frame};
use model::flow::{Endpoint, Failure, Flow, Place};
use model::history::History;
use model::occurrence::{self, Occurrence};
use model::structure::Value;
use model::world::{self, World};
use std::collections::{BTreeMap, BTreeSet};

fn place(level: u64, position: u64) -> Place {
    Place::World(
        world::Identity(level * 2 + position),
        occurrence::Identity(position),
    )
}

fn relation(level: u64, pattern: u64) -> Flow {
    Flow {
        resource: (0..2)
            .map(|target| {
                (
                    place(level + 1, target),
                    (0..2)
                        .filter(|source| pattern & (1 << (target * 2 + source)) != 0)
                        .map(|source| place(level, source))
                        .collect(),
                )
            })
            .collect(),
        context: (0..2)
            .map(|target| {
                (
                    world::Identity((level + 1) * 2 + target),
                    (0..2)
                        .filter(|source| pattern & (1 << (target * 2 + source)) != 0)
                        .map(|source| world::Identity(level * 2 + source))
                        .collect(),
                )
            })
            .collect(),
        frame: BTreeMap::new(),
    }
}

fn edge(pattern: u64, source: u64, target: u64) -> bool {
    pattern & (1 << (target * 2 + source)) != 0
}

#[test]
fn matrix() {
    for first in 0..16 {
        for second in 0..16 {
            for third in 0..16 {
                let left = relation(0, first);
                let middle = relation(1, second);
                let right = relation(2, third);
                let result = left.compose(&middle).unwrap().compose(&right).unwrap();
                assert_eq!(
                    result,
                    left.compose(&middle.compose(&right).unwrap()).unwrap()
                );
                for target in 0..2 {
                    let mut expected = BTreeSet::new();
                    for source in 0..2 {
                        for before in 0..2 {
                            for after in 0..2 {
                                if edge(first, source, before)
                                    && edge(second, before, after)
                                    && edge(third, after, target)
                                {
                                    expected.insert(source);
                                }
                            }
                        }
                    }
                    assert_eq!(
                        result.resource[&place(3, target)],
                        expected.iter().map(|&source| place(0, source)).collect()
                    );
                    assert_eq!(
                        result.context[&world::Identity(6 + target)],
                        expected.into_iter().map(world::Identity).collect()
                    );
                }
            }
        }
    }
}

fn frame(level: u64, pattern: u64) -> Flow {
    let mut flow = Flow {
        resource: BTreeMap::new(),
        context: BTreeMap::new(),
        frame: BTreeMap::new(),
    };
    for position in 0..2 {
        let choice = pattern / 3_u64.pow(position as u32) % 3;
        flow.frame.insert(
            context::Identity((level + 1) * 2 + position),
            (choice > 0).then(|| context::Identity(level * 2 + choice - 1)),
        );
    }
    flow
}

#[test]
fn allocation() {
    for first in 0..9 {
        for second in 0..9 {
            for third in 0..9 {
                let left = frame(0, first);
                let middle = frame(1, second);
                let right = frame(2, third);
                let result = left.compose(&middle).unwrap().compose(&right).unwrap();
                assert_eq!(
                    result,
                    left.compose(&middle.compose(&right).unwrap()).unwrap()
                );
                for target in 0..2 {
                    let mut source = Some(target);
                    for pattern in [third, second, first] {
                        source = source.and_then(|position| {
                            let choice = pattern / 3_u64.pow(position as u32) % 3;
                            (choice > 0).then(|| choice - 1)
                        });
                    }
                    assert_eq!(
                        result.frame[&context::Identity(6 + target)],
                        source.map(context::Identity)
                    );
                }
            }
        }
    }
}

fn state() -> Configuration {
    let occurrence = Occurrence {
        identity: occurrence::Identity(0),
        value: Value::Atom("A".into()),
        history: History::default(),
    };
    Configuration::new(
        context::Identity(0),
        vec![
            World {
                identity: world::Identity(0),
                context: context::Identity(0),
                occurrence: vec![occurrence.clone()],
            },
            World {
                identity: world::Identity(1),
                context: context::Identity(0),
                occurrence: vec![],
            },
        ],
        vec![Frame {
            identity: context::Identity(0),
            parent: None,
            lexical: None,
            declaration: vec![],
            held: vec![occurrence],
        }],
        History::default(),
    )
    .unwrap()
}

#[test]
fn identity() {
    let state = state();
    let identity = Flow::identity(&state);
    identity.validate(&state, &state).unwrap();
    assert_eq!(identity.compose(&identity).unwrap(), identity);
    assert_eq!(
        identity.context[&world::Identity(1)],
        BTreeSet::from([world::Identity(1)])
    );
    assert_eq!(
        identity
            .project(&BTreeSet::from([Place::Held(
                context::Identity(0),
                occurrence::Identity(0)
            )]))
            .unwrap(),
        BTreeSet::from([Place::Held(context::Identity(0), occurrence::Identity(0))])
    );
}

#[test]
fn invalid() {
    let state = state();
    let valid = Flow::identity(&state);
    let mut flow = valid.clone();
    flow.resource.remove(&place(0, 0));
    assert_eq!(
        flow.validate(&state, &state),
        Err(Failure::Resource {
            endpoint: Endpoint::Target,
            place: place(0, 0)
        })
    );
    assert_eq!(
        flow.compose(&valid),
        Err(Failure::Resource {
            endpoint: Endpoint::Target,
            place: place(0, 0)
        })
    );
    let mut flow = valid.clone();
    flow.resource
        .get_mut(&place(0, 0))
        .unwrap()
        .insert(place(7, 0));
    assert_eq!(
        flow.validate(&state, &state),
        Err(Failure::Resource {
            endpoint: Endpoint::Source,
            place: place(7, 0)
        })
    );
    let mut flow = valid.clone();
    flow.context.remove(&world::Identity(1));
    assert_eq!(
        flow.validate(&state, &state),
        Err(Failure::World {
            endpoint: Endpoint::Target,
            identity: world::Identity(1)
        })
    );
    assert!(flow.compose(&valid).is_err());
    let mut flow = valid.clone();
    flow.context
        .get_mut(&world::Identity(1))
        .unwrap()
        .insert(world::Identity(7));
    assert_eq!(
        flow.validate(&state, &state),
        Err(Failure::World {
            endpoint: Endpoint::Source,
            identity: world::Identity(7)
        })
    );
    let mut flow = valid.clone();
    flow.frame.remove(&context::Identity(0));
    assert_eq!(
        flow.validate(&state, &state),
        Err(Failure::Frame {
            endpoint: Endpoint::Target,
            identity: context::Identity(0)
        })
    );
    assert!(flow.compose(&valid).is_err());
    let mut flow = valid;
    flow.frame
        .insert(context::Identity(0), Some(context::Identity(7)));
    assert_eq!(
        flow.validate(&state, &state),
        Err(Failure::Frame {
            endpoint: Endpoint::Source,
            identity: context::Identity(7)
        })
    );
}
