use model::application::{Code, Request, Selection};
use model::capture::Capture;
use model::configuration::Configuration;
use model::construction::Construction;
use model::context;
use model::failure::Failure;
use model::flow::Place;
use model::fragment::Fragment;
use model::history::History;
use model::occurrence::Identity;
use model::path::{Path, Step};
use model::publication;
use model::qualification::Qualification;
use model::structure::Value;
use model::support::{self, Address};
use model::world;
use std::collections::BTreeSet;

pub(super) fn validate(path: &Path) {
    for state in path.state() {
        Configuration::new(
            state.root(),
            state.world().cloned().collect(),
            state.frame().cloned().collect(),
            state.history().clone(),
        )
        .unwrap();
    }
    assert_eq!(
        Path::replay(
            path.source().clone(),
            path.record().iter().map(|record| record.step.clone())
        )
        .unwrap(),
        *path
    );
}

pub(super) fn wrapper() -> (Path, Fragment<Value<Capture, Capture>>) {
    let (path, witness) = super::qualification::branch();
    let world = path.target().world().next().unwrap();
    let qualification = Qualification::new(path.clone(), world.identity).unwrap();
    let construction = qualification.construction();
    let value = construction
        .rule(
            construction
                .input([construction
                    .particle([construction.literal("Tick")])
                    .unwrap()])
                .unwrap(),
            construction
                .output([construction
                    .destination(
                        construction
                            .particle(
                                witness
                                    .into_iter()
                                    .map(|request| qualification.inspect(request).unwrap()),
                            )
                            .unwrap(),
                        None,
                    )
                    .unwrap()])
                .unwrap(),
        )
        .unwrap();
    (path, value)
}

pub(super) fn publish(path: &Path, value: Fragment<Value<Capture, Capture>>) -> Path {
    path.advance(Step::Construction(publication::Request {
        world: path.target().world().next().unwrap().identity,
        consumed: vec![],
        value,
    }))
    .unwrap()
}

fn request(path: &Path, code: Code, label: &str) -> Request {
    let world = path.target().world().next().unwrap();
    let value = world
        .occurrence
        .iter()
        .find(|value| value.value == Value::Atom(label.into()))
        .unwrap();
    Request {
        code,
        selection: vec![Selection {
            world: world.identity,
            occurrence: vec![value.identity],
        }],
    }
}

pub(super) fn opened(path: &Path) -> Request {
    let world = path.target().world().next().unwrap();
    let code = world.occurrence.iter().find(|value| matches!(&value.value, Value::Rule(rule) if rule.context == context::Identity(0))).unwrap();
    request(
        path,
        Code::Local {
            world: world.identity,
            occurrence: code.identity,
        },
        "Tick",
    )
}

pub(super) fn execute(path: &Path) {
    let world = path.target().world().next().unwrap();
    let mut observed = BTreeSet::new();
    for value in &world.occurrence {
        let Value::Rule(rule) = &value.value else {
            continue;
        };
        if rule.context == context::Identity(0) {
            continue;
        }
        let called = path
            .advance(Step::Application(request(
                path,
                Code::Local {
                    world: world.identity,
                    occurrence: value.identity,
                },
                "Call",
            )))
            .unwrap();
        let result = called
            .advance(Step::Application(request(
                &called,
                Code::Declaration {
                    context: rule.context,
                    position: 1,
                },
                "Secret",
            )))
            .unwrap();
        let world = result.target().world().next().unwrap();
        let output = world
            .occurrence
            .iter()
            .max_by_key(|value| value.identity)
            .unwrap();
        let Value::Atom(label) = &output.value else {
            panic!()
        };
        observed.insert(label.clone());
        assert_eq!(
            result.flow().resource[&Place::World(world.identity, output.identity)],
            BTreeSet::from([Place::World(world::Identity(0), Identity(1))])
        );
        validate(&result);
    }
    assert_eq!(observed, BTreeSet::from(["Left".into(), "Right".into()]));
}

#[test]
fn execution() {
    let (initial, value) = wrapper();
    let path = publish(&initial, value);
    let record = path.record().last().unwrap();
    assert!(record.read.is_empty());
    assert!(record.consumed.is_empty());
    assert_eq!(record.archive.len(), 2);
    for frame in path
        .target()
        .frame()
        .filter(|frame| frame.identity != context::Identity(0))
    {
        assert_eq!(frame.held.len(), 1);
        assert_eq!(frame.held[0].identity, Identity(0));
        assert_eq!(
            record.flow.resource[&Place::Held(frame.identity, Identity(0))],
            BTreeSet::from([Place::World(world::Identity(2), Identity(0))])
        );
        assert_eq!(
            path.flow().resource[&Place::Held(frame.identity, Identity(0))],
            BTreeSet::from([Place::World(world::Identity(0), Identity(0))])
        );
    }
    let expanded = path.advance(Step::Application(opened(&path))).unwrap();
    execute(&expanded);
    let inferred = Path::new(initial.source().clone())
        .advance(Step::Inference {
            request: opened(&path),
            path: Box::new(path),
        })
        .unwrap();
    assert_eq!(inferred.record()[0].archive.len(), 2);
    for origin in inferred.record()[0].archive.values() {
        assert_eq!(origin.address.derivation.len(), 2);
        assert_eq!(origin.address.derivation[0], 0);
    }
    execute(&inferred);
    let world = inferred.target().world().next().unwrap();
    let qualification = Qualification::new(inferred.clone(), world.identity).unwrap();
    for code in &world.occurrence {
        let Value::Rule(rule) = &code.value else {
            continue;
        };
        let value = qualification
            .inspect(support::Request {
                address: Address {
                    derivation: vec![],
                    state: 1,
                },
                world: world.identity,
                place: Place::World(world.identity, code.identity),
            })
            .unwrap();
        let Value::Rule(qualified) = value.value() else {
            panic!()
        };
        assert_eq!(
            qualified.context.address(),
            &inferred.record()[0].archive[&rule.context].address
        );
    }
}

#[test]
fn reclamation() {
    let (initial, value) = wrapper();
    let published = publish(&initial, value.clone());
    let world = published.target().world().next().unwrap();
    let code = world
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap();
    let erased = published
        .advance(Step::Introduction {
            world: world.identity,
            consumed: vec![code.identity],
            value: Construction::new(context::Identity(0), History::default()).literal("Bridge"),
        })
        .unwrap();
    assert_eq!(erased.target().frame().count(), 1);
    let restored = publish(&erased, value);
    assert_eq!(restored.target().frame().count(), 3);
    for (&identity, origin) in &restored.record().last().unwrap().archive {
        assert!(identity.0 > 2);
        assert_eq!(origin.address.state, 1);
        assert_eq!(origin.address.derivation.len(), 1);
    }
    let expanded = restored
        .advance(Step::Application(opened(&restored)))
        .unwrap();
    execute(&expanded);
    let inferred = Path::new(initial.source().clone())
        .advance(Step::Inference {
            request: opened(&restored),
            path: Box::new(restored),
        })
        .unwrap();
    execute(&inferred);
}

#[test]
fn ownership() {
    let path = super::support::captured();
    let qualification = Qualification::new(path.clone(), world::Identity(1)).unwrap();
    let value = qualification
        .inspect(support::Request {
            address: Address {
                derivation: vec![0],
                state: 2,
            },
            world: world::Identity(2),
            place: Place::World(world::Identity(2), Identity(3)),
        })
        .unwrap();
    let step = publication::Request {
        world: world::Identity(1),
        consumed: vec![],
        value,
    };
    let mut invalid = step.clone();
    invalid.consumed.push(Identity(1));
    assert_eq!(
        path.advance(Step::Construction(invalid)),
        Err(Failure::Occurrence(Identity(1)))
    );
    let result = path.advance(Step::Construction(step)).unwrap();
    let world = result.target().world().next().unwrap();
    assert_eq!(world.context, context::Identity(1));
    let code = world
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap();
    let Value::Rule(rule) = &code.value else {
        panic!()
    };
    assert_eq!(rule.context, context::Identity(2));
    assert_eq!(
        result.target().frame().nth(1).unwrap(),
        path.target().frame().nth(1).unwrap()
    );
    let held = qualification
        .inspect(support::Request {
            address: Address {
                derivation: vec![],
                state: 1,
            },
            world: world::Identity(1),
            place: Place::Held(context::Identity(1), Identity(1)),
        })
        .unwrap();
    let observed = publish(&path, held);
    assert_eq!(
        observed.record().last().unwrap().read,
        BTreeSet::from([Place::Held(context::Identity(1), Identity(1))])
    );
    assert!(observed.record().last().unwrap().consumed.is_empty());
    validate(&result);
    validate(&observed);
}

#[test]
fn capacity() {
    for reclaimed in [false, true] {
        let initial = super::archive::initial(false);
        let mut frame = initial.source().frame().cloned().collect::<Vec<_>>();
        frame.push(model::context::Frame {
            identity: context::Identity(u64::MAX),
            parent: Some(context::Identity(0)),
            lexical: Some(context::Identity(0)),
            declaration: vec![],
            held: vec![],
        });
        let initial = Path::new(
            Configuration::new(
                context::Identity(0),
                initial.source().world().cloned().collect(),
                frame,
                History::default(),
            )
            .unwrap(),
        );
        let qualification = Qualification::new(initial.clone(), world::Identity(0)).unwrap();
        let value = qualification
            .inspect(support::Request {
                address: Address::default(),
                world: world::Identity(0),
                place: Place::World(world::Identity(0), Identity(0)),
            })
            .unwrap();
        let path = if reclaimed {
            super::archive::remove(&initial, vec![Identity(0)])
        } else {
            initial
        };
        let original = path.clone();
        let result = path.advance(Step::Construction(publication::Request {
            world: path.target().world().next().unwrap().identity,
            consumed: vec![],
            value,
        }));
        if reclaimed {
            assert_eq!(result, Err(Failure::Capacity));
        } else {
            validate(&result.unwrap());
        }
        assert_eq!(path, original);
    }
}

#[test]
fn lineage() {
    let (initial, _) = wrapper();
    let mut world = initial.target().world().cloned().collect::<Vec<_>>();
    world.push(model::world::World {
        identity: world::Identity(9),
        context: context::Identity(0),
        occurrence: vec![],
    });
    let path = Path::new(
        Configuration::new(
            context::Identity(0),
            world,
            initial.target().frame().cloned().collect(),
            History::default(),
        )
        .unwrap(),
    );
    let qualification = Qualification::new(path.clone(), world::Identity(2)).unwrap();
    let request = publication::Request {
        world: world::Identity(9),
        consumed: vec![],
        value: qualification.construction().literal("A"),
    };
    assert_eq!(
        path.advance(Step::Construction(request.clone())),
        Err(Failure::Lineage {
            source: world::Identity(2),
            target: world::Identity(9)
        })
    );
    let result = path
        .advance(Step::Construction(publication::Request {
            world: world::Identity(2),
            ..request
        }))
        .unwrap();
    validate(&result);
}

#[test]
fn rejection() {
    let (path, value) = wrapper();
    let request = publication::Request {
        world: world::Identity(2),
        consumed: vec![],
        value,
    };
    assert_eq!(
        Path::new(path.target().clone()).advance(Step::Construction(request.clone())),
        Err(Failure::Source)
    );
    let mut absent = request.clone();
    absent.world = world::Identity(99);
    assert_eq!(
        path.advance(Step::Construction(absent)),
        Err(Failure::World(world::Identity(99)))
    );
    let mut repeated = request.clone();
    repeated.consumed = vec![Identity(0), Identity(0)];
    assert_eq!(
        path.advance(Step::Construction(repeated)),
        Err(Failure::Repeated(Identity(0)))
    );
    let mut consumed = request;
    consumed.consumed = vec![Identity(4)];
    let result = path.advance(Step::Construction(consumed)).unwrap();
    assert_eq!(
        result.record().last().unwrap().consumed,
        BTreeSet::from([Place::World(world::Identity(2), Identity(4))])
    );
    let world = result.target().world().next().unwrap();
    let code = world
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap();
    assert_eq!(
        result.flow().resource[&Place::World(world.identity, code.identity)],
        BTreeSet::from([Place::World(world::Identity(0), Identity(2))])
    );
    validate(&result);
}
