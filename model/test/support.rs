use model::application::{Code, Request, Selection};
use model::configuration::Configuration;
use model::context::{self, Frame};
use model::failure::Failure;
use model::flow::Place;
use model::history::History;
use model::occurrence::Identity;
use model::path::{Path, Step};
use model::structure::{Body, Destination, Input, Output, Particle, Rule, Value};
use model::support::{self, Address};
use model::world::{self, World};
use std::collections::BTreeSet;

fn rule(input: &str, output: &str) -> Rule {
    let Value::Rule(rule) = super::rule(0, input, Value::Atom(output.into())) else {
        unreachable!()
    };
    *rule
}

fn initial() -> Path {
    Path::new(
        Configuration::new(
            context::Identity(0),
            vec![World {
                identity: world::Identity(0),
                context: context::Identity(0),
                occurrence: vec![
                    super::source(0, Value::Atom("Seed".into())),
                    super::source(1, super::rule(0, "A", Value::Atom("B".into()))),
                ],
            }],
            vec![Frame {
                identity: context::Identity(0),
                parent: None,
                lexical: None,
                declaration: vec![
                    rule("Seed", "A"),
                    rule("A", "C"),
                    rule("B", "E"),
                    rule("E", "F"),
                    rule("F", "G"),
                ],
                held: vec![],
            }],
            History::default(),
        )
        .unwrap(),
    )
}

fn request(position: usize, world: u64, occurrence: u64) -> Request {
    Request {
        code: Code::Declaration {
            context: context::Identity(0),
            position,
        },
        selection: vec![Selection {
            world: world::Identity(world),
            occurrence: vec![Identity(occurrence)],
        }],
    }
}

pub(super) fn derivation() -> Path {
    let initial = initial();
    let deepest = initial
        .advance(Step::Application(request(0, 0, 0)))
        .unwrap();
    let middle = initial
        .advance(Step::Inference {
            path: Box::new(deepest),
            request: Request {
                code: Code::Local {
                    world: world::Identity(1),
                    occurrence: Identity(1),
                },
                ..request(0, 1, 2)
            },
        })
        .unwrap()
        .advance(Step::Application(request(2, 1, 2)))
        .unwrap();
    initial
        .advance(Step::Inference {
            path: Box::new(middle),
            request: request(3, 2, 3),
        })
        .unwrap()
        .advance(Step::Application(request(4, 1, 2)))
        .unwrap()
}

fn witness(derivation: Vec<usize>, state: usize, world: u64, occurrence: u64) -> support::Request {
    support::Request {
        address: Address { derivation, state },
        world: world::Identity(world),
        place: Place::World(world::Identity(world), Identity(occurrence)),
    }
}

#[test]
fn nested() {
    let path = derivation();
    for (derivation, state, world, occurrence, expected) in [
        (vec![], 0, 0, 0, "Seed"),
        (vec![], 1, 1, 2, "F"),
        (vec![], 2, 2, 3, "G"),
        (vec![0], 0, 0, 0, "Seed"),
        (vec![0], 1, 1, 2, "B"),
        (vec![0], 2, 2, 3, "E"),
        (vec![0, 0], 0, 0, 0, "Seed"),
        (vec![0, 0], 1, 1, 2, "A"),
    ] {
        let request = witness(derivation, state, world, occurrence);
        let support = support::resolve(&path, world::Identity(2), &request).unwrap();
        assert_eq!(support.path(), &path);
        assert_eq!(support.world(), world::Identity(2));
        assert_eq!(support.request(), &request);
        assert_eq!(support.occurrence().value, Value::Atom(expected.into()));
        assert_eq!(
            support.flow().resource[&request.place],
            BTreeSet::from([Place::World(world::Identity(0), Identity(0))])
        );
        support
            .flow()
            .validate(path.source(), support.state())
            .unwrap();
        assert_eq!(
            support.read(),
            &if request.address.derivation.is_empty() && state == 2 {
                BTreeSet::from([request.place])
            } else {
                BTreeSet::new()
            }
        );
    }
    let Step::Inference { path: middle, .. } = &path.record()[0].step else {
        panic!()
    };
    assert_eq!(
        middle.record()[0].read,
        BTreeSet::from([Place::World(world::Identity(0), Identity(1))])
    );
    assert!(path.record()[0].read.is_empty());
    assert_eq!(
        Path::replay(
            path.source().clone(),
            path.record().iter().map(|record| record.step.clone())
        )
        .unwrap(),
        path
    );
}

#[test]
fn prefix() {
    let prefix = initial()
        .advance(Step::Application(request(0, 0, 0)))
        .unwrap();
    let auxiliary = Path::new(prefix.target().clone())
        .advance(Step::Application(Request {
            code: Code::Local {
                world: world::Identity(1),
                occurrence: Identity(1),
            },
            ..request(0, 1, 2)
        }))
        .unwrap();
    let path = prefix
        .advance(Step::Inference {
            path: Box::new(auxiliary),
            request: request(2, 2, 3),
        })
        .unwrap();
    let request = witness(vec![1], 1, 2, 3);
    let support = support::resolve(&path, world::Identity(2), &request).unwrap();
    assert_eq!(support.occurrence().value, Value::Atom("B".into()));
    assert_eq!(
        support.flow().resource[&request.place],
        BTreeSet::from([Place::World(world::Identity(0), Identity(0))])
    );
    assert!(support.read().is_empty());
}

#[test]
fn rejection() {
    let path = derivation();
    for (derivation, depth, position) in [
        (vec![99], 0, 99),
        (vec![1], 0, 1),
        (vec![0, 1], 1, 1),
        (vec![0, 0, 0], 2, 0),
    ] {
        let request = witness(derivation, 0, 0, 0);
        assert_eq!(
            support::resolve(&path, world::Identity(2), &request).unwrap_err(),
            Failure::Derivation { depth, position }
        );
    }
    let mut location = witness(vec![0, 0], 2, 1, 2);
    assert_eq!(
        support::resolve(&path, world::Identity(2), &location).unwrap_err(),
        Failure::State(2)
    );
    location.address.state = 1;
    location.place = Place::World(world::Identity(1), Identity(99));
    assert_eq!(
        support::resolve(&path, world::Identity(2), &location).unwrap_err(),
        Failure::Occurrence(Identity(99))
    );
    location.place = Place::World(world::Identity(0), Identity(0));
    assert_eq!(
        support::resolve(&path, world::Identity(2), &location).unwrap_err(),
        Failure::World(world::Identity(0))
    );
    assert_eq!(
        support::resolve(&path, world::Identity(99), &location).unwrap_err(),
        Failure::World(world::Identity(99))
    );
    let initial = initial();
    let mut world = initial.source().world().cloned().collect::<Vec<_>>();
    world.push(World {
        identity: world::Identity(9),
        context: context::Identity(0),
        occurrence: vec![super::source(9, Value::Atom("Other".into()))],
    });
    let initial = Path::new(
        Configuration::new(
            context::Identity(0),
            world,
            initial.source().frame().cloned().collect(),
            History::default(),
        )
        .unwrap(),
    );
    let auxiliary = initial
        .advance(Step::Application(request(0, 0, 0)))
        .unwrap();
    let path = initial
        .advance(Step::Inference {
            path: Box::new(auxiliary),
            request: request(1, 10, 10),
        })
        .unwrap();
    let request = witness(vec![0], 1, 10, 10);
    assert_eq!(
        support::resolve(&path, world::Identity(9), &request).unwrap_err(),
        Failure::Lineage {
            source: world::Identity(0),
            target: world::Identity(9),
        }
    );
}

#[test]
fn conjunction() {
    let mut joined = rule("Seed", "A");
    joined.input = Input::new(vec![
        Particle::new(vec![Value::Atom("Seed".into())]),
        Particle::new(vec![Value::Atom("Other".into())]),
    ]);
    let initial = Path::new(
        Configuration::new(
            context::Identity(0),
            [(0, "Seed"), (9, "Other"), (20, "Keep")]
                .into_iter()
                .map(|(identity, label)| World {
                    identity: world::Identity(identity),
                    context: context::Identity(0),
                    occurrence: vec![super::source(identity, Value::Atom(label.into()))],
                })
                .collect(),
            vec![Frame {
                identity: context::Identity(0),
                parent: None,
                lexical: None,
                declaration: vec![joined, rule("Keep", "Done")],
                held: vec![],
            }],
            History::default(),
        )
        .unwrap(),
    );
    let auxiliary = initial
        .advance(Step::Application(Request {
            selection: [9, 0]
                .into_iter()
                .map(|identity| Selection {
                    world: world::Identity(identity),
                    occurrence: vec![Identity(identity)],
                })
                .collect(),
            ..request(0, 0, 0)
        }))
        .unwrap();
    let path = initial
        .advance(Step::Inference {
            path: Box::new(auxiliary),
            request: request(1, 20, 20),
        })
        .unwrap();
    for (target, missing) in [(0, 9), (9, 0), (21, 0)] {
        let request = witness(vec![0], 1, 21, 21);
        assert_eq!(
            support::resolve(&path, world::Identity(target), &request).unwrap_err(),
            Failure::Lineage {
                source: world::Identity(missing),
                target: world::Identity(target),
            }
        );
    }
    let request = witness(vec![0], 1, 20, 20);
    let support = support::resolve(&path, world::Identity(21), &request).unwrap();
    assert_eq!(support.occurrence().value, Value::Atom("Keep".into()));
}

#[test]
fn archive() {
    let initial = super::archive::initial(false);
    let value = super::archive::retain(&initial, Identity(0));
    let erased = super::archive::remove(&initial, vec![Identity(0)]);
    let auxiliary = super::archive::restore(&erased, value, 0, Identity(0));
    let world = auxiliary.target().world().next().unwrap();
    let code = world
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap();
    let location = witness(vec![0], 2, world.identity.0, code.identity.0);
    let request = Request {
        code: Code::Local {
            world: world.identity,
            occurrence: code.identity,
        },
        selection: vec![Selection {
            world: world.identity,
            occurrence: vec![Identity(1)],
        }],
    };
    let path = initial
        .advance(Step::Inference {
            path: Box::new(auxiliary),
            request,
        })
        .unwrap();
    let target = path.target().world().next().unwrap().identity;
    let support = support::resolve(&path, target, &location).unwrap();
    let frame = support.state().frame().nth(1).unwrap();
    assert_eq!(
        support.flow().frame[&frame.identity],
        Some(context::Identity(1))
    );
    assert_eq!(
        support.flow().resource[&Place::Held(frame.identity, frame.held[0].identity)],
        BTreeSet::from([Place::Held(context::Identity(1), Identity(10))])
    );
    assert!(support.read().is_empty());
}

pub(super) fn captured() -> Path {
    let local = model::context::Reference::Local(0);
    let mut producer = model::activation::capture(rule("Make", "Unused"));
    producer.context = local;
    let mut closure = model::activation::capture(rule("Call", "Private"));
    closure.context = local;
    producer.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Rule(Box::new(closure))]),
        body: None,
    }]);
    let mut enter = rule("Enter", "Make");
    enter.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Atom("Make".into())]),
        body: Some(Body::bind(context::Identity(0), vec![producer]).unwrap()),
    }]);
    let mut call = rule("Call", "D");
    call.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Atom("D".into())]),
        body: Some(Body::new(context::Identity(0), vec![rule("A", "Global")])),
    }]);
    let initial = Path::new(
        Configuration::new(
            context::Identity(0),
            vec![World {
                identity: world::Identity(0),
                context: context::Identity(0),
                occurrence: vec![
                    super::source(0, Value::Atom("Enter".into())),
                    super::source(1, Value::Atom("Call".into())),
                ],
            }],
            vec![Frame {
                identity: context::Identity(0),
                parent: None,
                lexical: None,
                declaration: vec![enter, call],
                held: vec![],
            }],
            History::default(),
        )
        .unwrap(),
    );
    let auxiliary = initial
        .advance(Step::Application(request(0, 0, 0)))
        .unwrap()
        .advance(Step::Application(Request {
            code: Code::Declaration {
                context: context::Identity(1),
                position: 0,
            },
            ..request(0, 1, 2)
        }))
        .unwrap();
    initial
        .advance(Step::Inference {
            path: Box::new(auxiliary),
            request: request(1, 2, 1),
        })
        .unwrap()
}

#[test]
fn capture() {
    let path = captured();
    let request = witness(vec![0], 2, 2, 3);
    let support = support::resolve(&path, world::Identity(1), &request).unwrap();
    let Value::Rule(code) = &support.occurrence().value else {
        panic!()
    };
    assert_eq!(code.context, context::Identity(1));
    assert_eq!(support.flow().frame[&code.context], None);
    assert_ne!(
        support.state().frame().nth(1).unwrap(),
        path.target().frame().nth(1).unwrap()
    );
    assert_eq!(
        support.flow().resource[&request.place],
        BTreeSet::from([Place::World(world::Identity(0), Identity(0))])
    );
    let request = support::Request {
        address: Address {
            derivation: vec![0],
            state: 1,
        },
        world: world::Identity(1),
        place: Place::Held(context::Identity(1), Identity(0)),
    };
    let held = support::resolve(&path, world::Identity(1), &request).unwrap();
    assert_eq!(held.occurrence().value, Value::Atom("Enter".into()));
    assert!(held.read().is_empty());
    assert_eq!(
        held.flow().resource[&request.place],
        BTreeSet::from([Place::World(world::Identity(0), Identity(0))])
    );
    let request = support::Request {
        address: Address {
            derivation: vec![0],
            state: 2,
        },
        world: world::Identity(2),
        ..request
    };
    assert_eq!(
        support::resolve(&path, world::Identity(1), &request).unwrap_err(),
        Failure::Owner {
            world: world::Identity(2),
            context: context::Identity(1),
        }
    );
}
