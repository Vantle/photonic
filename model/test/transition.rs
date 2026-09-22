use model::application::{Code, Request, Selection};
use model::comparison;
use model::configuration::Configuration;
use model::context::{self, Frame};
use model::flow::{Flow, Place};
use model::history::History;
use model::occurrence::{self, Occurrence};
use model::path::{Path, Step};
use model::provenance;
use model::structure::{Body, Destination, Output, Particle, Rule, Value};
use model::transition::{self, Transition};
use model::world::{self, World};
use std::collections::BTreeSet;

fn rule(context: u64, input: &str, output: &str) -> Rule {
    let Value::Rule(rule) = super::rule(context, input, Value::Atom(output.into())) else {
        panic!()
    };
    *rule
}

fn initial(context: u64, world: u64, occurrence: [u64; 4], reverse: bool) -> Path {
    let mut declaration = vec![
        rule(context, "A", "B"),
        rule(context, "A", "C"),
        rule(context, "A", "A"),
    ];
    if reverse {
        declaration.reverse();
    }
    Path::new(
        Configuration::new(
            context::Identity(context),
            vec![World {
                identity: world::Identity(world),
                context: context::Identity(context),
                occurrence: [
                    Value::Atom("A".into()),
                    Value::Atom("A".into()),
                    Value::Rule(Box::new(rule(context, "A", "B"))),
                    Value::Rule(Box::new(rule(context, "A", "B"))),
                ]
                .into_iter()
                .zip(occurrence)
                .map(|(value, identity)| Occurrence {
                    identity: occurrence::Identity(identity),
                    value,
                    history: History::default(),
                })
                .collect(),
            }],
            vec![Frame {
                identity: context::Identity(context),
                parent: None,
                lexical: None,
                declaration,
                held: vec![],
            }],
            History::default(),
        )
        .unwrap(),
    )
}

fn request(path: &Path, code: Code, occurrence: u64) -> Request {
    Request {
        code,
        selection: vec![Selection {
            world: path.target().world().next().unwrap().identity,
            occurrence: vec![occurrence::Identity(occurrence)],
        }],
    }
}

fn local(path: &Path, code: u64, occurrence: u64) -> Transition {
    Transition::new(
        path.clone(),
        request(
            path,
            Code::Local {
                world: path.target().world().next().unwrap().identity,
                occurrence: occurrence::Identity(code),
            },
            occurrence,
        ),
    )
    .unwrap()
}

fn same(source: &Transition, target: &Transition) {
    let mapping = transition::boundary(source, target).unwrap();
    assert!(provenance::compare(
        mapping.source(),
        mapping.witness(),
        source.path().flow(),
        target.path().flow()
    ));
    assert!(provenance::compare(
        mapping.source(),
        mapping.target(),
        &source.event().flow,
        &target.event().flow
    ));
    assert!(transition::boundary(target, source).is_some());
}

#[test]
fn renaming() {
    let source = initial(0, 0, [0, 1, 2, 3], false);
    let target = initial(7, 9, [70, 30, 40, 60], true);
    same(&local(&source, 2, 0), &local(&target, 40, 70));
    let source = Transition::new(
        source.clone(),
        request(
            &source,
            Code::Declaration {
                context: context::Identity(0),
                position: 0,
            },
            0,
        ),
    )
    .unwrap();
    let target = Transition::new(
        target.clone(),
        request(
            &target,
            Code::Declaration {
                context: context::Identity(7),
                position: 2,
            },
            70,
        ),
    )
    .unwrap();
    same(&source, &target);
    assert_ne!(source.request().code, target.request().code);
}

#[test]
fn ambiguity() {
    let source = initial(0, 0, [0, 1, 2, 3], false);
    let left = local(&source, 2, 0);
    let right = local(&source, 3, 1);
    assert!(comparison::compare(&left.event().target, &right.event().target).is_some());
    let mapping = transition::boundary(&left, &right).unwrap();
    assert_eq!(
        mapping.source().occurrence()[&occurrence::Identity(0)],
        occurrence::Identity(1)
    );
    assert_eq!(
        mapping.source().occurrence()[&occurrence::Identity(2)],
        occurrence::Identity(3)
    );
    assert!(
        transition::find(&left, &right, |mapping| mapping
            .source()
            .occurrence()
            .iter()
            .all(|(left, right)| left == right))
        .is_none()
    );
    same(&left, &right);
}

#[test]
fn read() {
    let source = initial(0, 0, [0, 1, 2, 3], false);
    let left = local(&source, 2, 0);
    let right = Transition::new(
        source.clone(),
        request(
            &source,
            Code::Declaration {
                context: context::Identity(0),
                position: 0,
            },
            0,
        ),
    )
    .unwrap();
    assert_eq!(left.event().target, right.event().target);
    assert_eq!(left.event().flow, right.event().flow);
    assert_eq!(left.event().consumed, right.event().consumed);
    assert_ne!(left.event().read, right.event().read);
    assert!(transition::boundary(&left, &right).is_none());
    assert!(transition::boundary(&right, &left).is_none());
}

#[test]
fn flow() {
    let source = initial(0, 0, [0, 1, 2, 3], false);
    let target = initial(7, 9, [70, 30, 40, 60], true);
    let left = Flow::identity(source.source());
    let right = Flow::identity(target.source());
    let mapping = comparison::compare(source.source(), target.source()).unwrap();
    assert!(provenance::compare(&mapping, &mapping, &left, &right));
    let place = Place::World(world::Identity(9), occurrence::Identity(70));
    let mut changed = right.clone();
    changed.resource.insert(place, BTreeSet::new());
    assert!(changed.validate(target.source(), target.source()).is_ok());
    assert!(!provenance::compare(&mapping, &mapping, &left, &changed));
    let mut changed = right.clone();
    changed.context.insert(world::Identity(9), BTreeSet::new());
    assert!(!provenance::compare(&mapping, &mapping, &left, &changed));
    let mut changed = right.clone();
    changed.frame.insert(context::Identity(7), None);
    assert!(!provenance::compare(&mapping, &mapping, &left, &changed));
    let mut changed = right;
    changed.resource.remove(&place);
    assert!(!provenance::compare(&mapping, &mapping, &left, &changed));
    assert_eq!(
        provenance::place(
            &mapping,
            Place::World(world::Identity(99), occurrence::Identity(0))
        ),
        None
    );
}

#[test]
fn inference() {
    let mut result = Vec::new();
    for (context, world, occurrence, reverse) in
        [(0, 0, [0, 1, 2, 3], false), (7, 9, [70, 30, 40, 60], true)]
    {
        let source = initial(context, world, occurrence, reverse);
        let identity = Request {
            code: Code::Declaration {
                context: context::Identity(context),
                position: if reverse { 0 } else { 2 },
            },
            selection: vec![Selection {
                world: world::Identity(world),
                occurrence: vec![occurrence::Identity(occurrence[0])],
            }],
        };
        let support = source.advance(Step::Application(identity.clone())).unwrap();
        let world = support.target().world().next().unwrap();
        let produced = world
            .occurrence
            .iter()
            .max_by_key(|value| value.identity)
            .unwrap();
        let request = request(&support, identity.code, produced.identity.0);
        let nested = source
            .advance(Step::Inference {
                path: Box::new(support),
                request,
            })
            .unwrap();
        let produced = nested
            .target()
            .world()
            .next()
            .unwrap()
            .occurrence
            .iter()
            .max_by_key(|value| value.identity)
            .unwrap()
            .identity;
        result.push(local(&nested, occurrence[2], produced.0));
    }
    same(&result[0], &result[1]);
    assert!(
        model::derivation::compare(result[0].path(), result[1].path())
            .unwrap()
            .is_some()
    );
    assert_eq!(result[0].binding().exact.len(), 1);
    assert_eq!(result[0].binding().footprint.len(), 1);
}

#[test]
fn restoration() {
    for shared in [false, true] {
        let original = super::archive::initial(shared);
        let value = super::archive::retain(&original, occurrence::Identity(0));
        let direct = local(&original, 0, 1);
        let mut expected = None;
        let mut path = original;
        for _ in 0..3 {
            let world = path.target().world().next().unwrap();
            let code = world
                .occurrence
                .iter()
                .find(|value| matches!(value.value, Value::Rule(_)))
                .unwrap()
                .identity;
            let erased = super::archive::remove(&path, vec![code]);
            path = super::archive::restore(&erased, value.clone(), 0, occurrence::Identity(0));
            let world = path.target().world().next().unwrap();
            let code = world
                .occurrence
                .iter()
                .find(|value| matches!(value.value, Value::Rule(_)))
                .unwrap()
                .identity;
            let restored = local(&path, code.0, 1);
            assert!(
                comparison::compare(direct.path().target(), restored.path().target()).is_some()
            );
            assert!(transition::boundary(&direct, &restored).is_none());
            if let Some(expected) = &expected {
                same(expected, &restored);
            }
            expected = Some(restored);
        }
    }
}

#[test]
fn scoped() {
    let mut result = Vec::new();
    for context in [0, 7] {
        let mut declaration = rule(context, "A", "B");
        declaration.output = Output::new(vec![Destination {
            particle: Particle::new(vec![Value::Atom("B".into())]),
            body: Some(Body::new(context::Identity(context), vec![])),
        }]);
        let source = Path::new(
            Configuration::new(
                context::Identity(context),
                vec![World {
                    identity: world::Identity(context),
                    context: context::Identity(context),
                    occurrence: vec![Occurrence {
                        identity: occurrence::Identity(context),
                        value: Value::Atom("A".into()),
                        history: History::default(),
                    }],
                }],
                vec![Frame {
                    identity: context::Identity(context),
                    parent: None,
                    lexical: None,
                    declaration: vec![declaration],
                    held: vec![],
                }],
                History::default(),
            )
            .unwrap(),
        );
        result.push(
            Transition::new(
                source.clone(),
                request(
                    &source,
                    Code::Declaration {
                        context: context::Identity(context),
                        position: 0,
                    },
                    context,
                ),
            )
            .unwrap(),
        );
    }
    same(&result[0], &result[1]);
    assert_eq!(result[0].binding().exact.len(), 1);
    let frame = result[0]
        .event()
        .target
        .frame()
        .find(|frame| frame.parent.is_some())
        .unwrap();
    assert_eq!(frame.held.len(), 1);
    let mapping = transition::boundary(&result[0], &result[1]).unwrap();
    let mut changed = result[1].event().flow.clone();
    let held = changed
        .resource
        .keys()
        .find(|place| matches!(place, Place::Held(_, _)))
        .copied()
        .unwrap();
    changed.resource.insert(held, BTreeSet::new());
    assert!(!provenance::compare(
        mapping.source(),
        mapping.target(),
        &result[0].event().flow,
        &changed
    ));
}

#[test]
fn support() {
    let initial = initial(0, 0, [0, 1, 2, 3], false);
    let local = initial
        .advance(Step::Application(request(
            &initial,
            Code::Local {
                world: world::Identity(0),
                occurrence: occurrence::Identity(2),
            },
            0,
        )))
        .unwrap();
    let declared = initial
        .advance(Step::Application(request(
            &initial,
            Code::Declaration {
                context: context::Identity(0),
                position: 0,
            },
            0,
        )))
        .unwrap();
    assert_eq!(local.target(), declared.target());
    assert_eq!(local.flow(), declared.flow());
    assert_ne!(local.record()[0].read, declared.record()[0].read);
    let request = request(
        &local,
        Code::Local {
            world: local.target().world().next().unwrap().identity,
            occurrence: occurrence::Identity(2),
        },
        1,
    );
    let left = Transition::new(local, request.clone()).unwrap();
    let right = Transition::new(declared, request).unwrap();
    same(&left, &right);
    assert_ne!(left.path(), right.path());
    assert_ne!(left, right);
    assert!(
        model::derivation::compare(left.path(), right.path())
            .unwrap()
            .is_none()
    );
    let left = left.retain().unwrap();
    let right = right.retain().unwrap();
    assert_eq!(left.target(), right.target());
    assert_eq!(left.flow(), right.flow());
    assert!(model::derivation::compare(&left, &right).unwrap().is_none());
}

#[test]
fn rejection() {
    let source = initial(0, 0, [0, 1, 2, 3], false);
    let invalid = request(
        &source,
        Code::Declaration {
            context: context::Identity(0),
            position: 99,
        },
        0,
    );
    assert_eq!(
        Transition::new(source.clone(), invalid),
        Err(model::failure::Failure::Declaration {
            context: context::Identity(0),
            position: 99
        })
    );
    let invalid = request(
        &source,
        Code::Local {
            world: world::Identity(0),
            occurrence: occurrence::Identity(99),
        },
        0,
    );
    assert_eq!(
        Transition::new(source.clone(), invalid),
        Err(model::failure::Failure::Occurrence(occurrence::Identity(
            99
        )))
    );
    let mut invalid = request(
        &source,
        Code::Local {
            world: world::Identity(0),
            occurrence: occurrence::Identity(2),
        },
        0,
    );
    invalid.selection[0].occurrence[0] = occurrence::Identity(99);
    assert_eq!(
        Transition::new(source, invalid),
        Err(model::failure::Failure::Occurrence(occurrence::Identity(
            99
        )))
    );
}

#[test]
fn incidence() {
    let source = initial(0, 0, [0, 1, 2, 3], false);
    let target = initial(7, 9, [70, 30, 40, 60], true);
    let mapping = comparison::find(source.source(), target.source(), |mapping| {
        mapping.occurrence()[&occurrence::Identity(0)] == occurrence::Identity(70)
    })
    .unwrap();
    let original = [
        Place::World(world::Identity(0), occurrence::Identity(0)),
        Place::World(world::Identity(0), occurrence::Identity(1)),
    ];
    let destination = [
        Place::World(world::Identity(9), occurrence::Identity(70)),
        Place::World(world::Identity(9), occurrence::Identity(30)),
    ];
    for left in 0..16 {
        let mut before = Flow::identity(source.source());
        for (row, &place) in original.iter().enumerate() {
            before.resource.insert(
                place,
                original
                    .iter()
                    .enumerate()
                    .filter(|(column, _)| left & (1 << (row * 2 + column)) != 0)
                    .map(|(_, &place)| place)
                    .collect(),
            );
        }
        before.validate(source.source(), source.source()).unwrap();
        for right in 0..16 {
            let mut after = Flow::identity(target.source());
            for (row, &place) in destination.iter().enumerate() {
                after.resource.insert(
                    place,
                    destination
                        .iter()
                        .enumerate()
                        .filter(|(column, _)| right & (1 << (row * 2 + column)) != 0)
                        .map(|(_, &place)| place)
                        .collect(),
                );
            }
            after.validate(target.source(), target.source()).unwrap();
            assert_eq!(
                provenance::compare(&mapping, &mapping, &before, &after),
                left == right
            );
        }
    }
}

#[test]
fn exact() {
    for changed in [false, true] {
        let mut result = Vec::new();
        for identity in [0, 7] {
            let label = if changed { "B" } else { "A" };
            let mut consumer = rule(identity, label, "D");
            consumer.output = Output::new(vec![Destination {
                particle: Particle::new(vec![Value::Atom("D".into())]),
                body: Some(Body::new(context::Identity(identity), vec![])),
            }]);
            let initial = Path::new(
                Configuration::new(
                    context::Identity(identity),
                    vec![World {
                        identity: world::Identity(identity),
                        context: context::Identity(identity),
                        occurrence: vec![Occurrence {
                            identity: occurrence::Identity(identity),
                            value: Value::Atom("A".into()),
                            history: History::default(),
                        }],
                    }],
                    vec![Frame {
                        identity: context::Identity(identity),
                        parent: None,
                        lexical: None,
                        declaration: vec![rule(identity, "A", label), consumer],
                        held: vec![],
                    }],
                    History::default(),
                )
                .unwrap(),
            );
            let prepared = initial
                .advance(Step::Application(request(
                    &initial,
                    Code::Declaration {
                        context: context::Identity(identity),
                        position: 0,
                    },
                    identity,
                )))
                .unwrap();
            let selected = prepared.target().world().next().unwrap().occurrence[0].identity;
            result.push(
                Transition::new(
                    prepared.clone(),
                    request(
                        &prepared,
                        Code::Declaration {
                            context: context::Identity(identity),
                            position: 1,
                        },
                        selected.0,
                    ),
                )
                .unwrap(),
            );
        }
        same(&result[0], &result[1]);
        assert_eq!(result[0].binding().footprint.len(), 1);
        assert_eq!(result[0].binding().exact.len(), usize::from(!changed));
        let frame = result[0]
            .event()
            .target
            .frame()
            .find(|frame| frame.parent.is_some())
            .unwrap();
        assert_eq!(frame.held.len(), usize::from(!changed));
        let world = result[0].event().target.world().next().unwrap();
        assert_eq!(
            world
                .occurrence
                .iter()
                .any(|value| value.value == Value::Atom("A".into())),
            changed
        );
    }
}
