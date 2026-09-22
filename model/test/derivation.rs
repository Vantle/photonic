use model::application::{Code, Request, Selection};
use model::configuration::Configuration;
use model::construction::Construction;
use model::context::{self, Frame};
use model::derivation::{self, Missing, Side, Unsupported};
use model::flow::Place;
use model::history::History;
use model::occurrence::{self, Occurrence};
use model::path::{Path, Step};
use model::qualification::Qualification;
use model::structure::{Body, Destination, Output, Particle, Value};
use model::support::{self, Address};
use model::world::{self, World};

fn fixture(base: u64) -> Path {
    let Value::Rule(mut code) = super::rule(base + 1, "Call", Value::Atom("Secret".into())) else {
        panic!()
    };
    code.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Atom("Secret".into())]),
        body: Some(Body::new(context::Identity(base + 1), vec![])),
    }]);
    let Value::Rule(declaration) = super::rule(base + 1, "Secret", Value::Atom("Private".into()))
    else {
        panic!()
    };
    let source = |identity, value| Occurrence {
        identity: occurrence::Identity(identity),
        value,
        history: History::default(),
    };
    Path::new(
        Configuration::new(
            context::Identity(base),
            vec![World {
                identity: world::Identity(base),
                context: context::Identity(base),
                occurrence: vec![
                    source(base, Value::Rule(code)),
                    source(base + 1, Value::Atom("Call".into())),
                ],
            }],
            vec![
                Frame {
                    identity: context::Identity(base),
                    parent: None,
                    lexical: None,
                    declaration: vec![],
                    held: vec![],
                },
                Frame {
                    identity: context::Identity(base + 1),
                    parent: Some(context::Identity(base)),
                    lexical: Some(context::Identity(base)),
                    declaration: vec![*declaration],
                    held: vec![source(base + 2, Value::Atom("Reserve".into()))],
                },
            ],
            History::default(),
        )
        .unwrap(),
    )
}

fn witness(path: &Path, state: usize) -> support::Request {
    let world = path.state()[state].world().next().unwrap();
    let code = world
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap();
    support::Request {
        address: Address {
            derivation: vec![],
            state,
        },
        world: world.identity,
        place: Place::World(world.identity, code.identity),
    }
}

fn erase(path: &Path) -> Path {
    let world = path.target().world().next().unwrap();
    let code = world
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap();
    path.advance(Step::Introduction {
        world: world.identity,
        consumed: vec![code.identity],
        value: Construction::new(path.target().root(), path.target().history().clone())
            .literal("Bridge"),
    })
    .unwrap()
}

fn publish(
    path: &Path,
    value: model::fragment::Fragment<Value<model::capture::Capture, model::capture::Capture>>,
) -> Path {
    let world = path.target().world().next().unwrap();
    path.advance(Step::Construction(model::publication::Request {
        world: world.identity,
        consumed: world
            .occurrence
            .iter()
            .filter(|value| value.value == Value::Atom("Bridge".into()))
            .map(|value| value.identity)
            .collect(),
        value,
    }))
    .unwrap()
}

fn request(path: &Path) -> Request {
    let world = path.target().world().next().unwrap();
    let code = world
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap();
    let call = world
        .occurrence
        .iter()
        .find(|value| value.value == Value::Atom("Call".into()))
        .unwrap();
    Request {
        code: Code::Local {
            world: world.identity,
            occurrence: code.identity,
        },
        selection: vec![Selection {
            world: world.identity,
            occurrence: vec![call.identity],
        }],
    }
}

fn same(left: &Path, right: &Path) {
    assert!(derivation::compare(left, right).unwrap().is_some());
    assert!(derivation::compare(right, left).unwrap().is_some());
    super::publication::validate(left);
    super::publication::validate(right);
}

#[test]
fn construction() {
    let build = |base| {
        let initial = fixture(base);
        let qualification = Qualification::new(initial.clone(), world::Identity(base)).unwrap();
        let value = qualification.inspect(witness(&initial, 0)).unwrap();
        let restored = publish(&erase(&initial), value);
        let executed = restored
            .advance(Step::Application(request(&restored)))
            .unwrap();
        let world = executed.target().world().next().unwrap();
        let secret = world
            .occurrence
            .iter()
            .find(|value| value.value == Value::Atom("Secret".into()))
            .unwrap();
        executed
            .advance(Step::Application(Request {
                code: Code::Declaration {
                    context: restored
                        .target()
                        .frame()
                        .find(|frame| frame.parent.is_some())
                        .unwrap()
                        .identity,
                    position: 0,
                },
                selection: vec![Selection {
                    world: world.identity,
                    occurrence: vec![secret.identity],
                }],
            }))
            .unwrap()
    };
    let left = build(0);
    let right = build(19);
    same(&left, &right);
    let mapping = derivation::compare(&left, &right).unwrap().unwrap();
    assert_eq!(
        mapping.state()[0].frame()[&context::Identity(1)],
        context::Identity(20)
    );
    assert_eq!(
        mapping.state()[2].frame()[&context::Identity(2)],
        context::Identity(21)
    );
    assert!(
        mapping
            .at(&Address {
                derivation: vec![0],
                state: 0
            })
            .is_none()
    );
}

#[test]
fn historical() {
    let build = |base| {
        let initial = fixture(base);
        let world = initial.target().world().next().unwrap();
        let source = &world.occurrence[0];
        let value = Construction::new(initial.target().root(), History::default())
            .inspect(source)
            .unwrap();
        let location = model::admission::Witness {
            state: 0,
            world: world.identity,
            place: Place::World(world.identity, source.identity),
        };
        let mut path = initial;
        for _ in 0..2 {
            let erased = erase(&path);
            let world = erased.target().world().next().unwrap();
            path = erased
                .advance(Step::Historical(model::admission::Request {
                    world: world.identity,
                    consumed: world
                        .occurrence
                        .iter()
                        .filter(|value| value.value == Value::Atom("Bridge".into()))
                        .map(|value| value.identity)
                        .collect(),
                    value: value.clone(),
                    witness: vec![location],
                }))
                .unwrap();
        }
        path
    };
    same(&build(0), &build(19));
}

#[test]
fn auxiliary() {
    let (initial, value) = super::publication::wrapper();
    let published = super::publication::publish(&initial, value);
    let inference = Path::new(initial.source().clone())
        .advance(Step::Inference {
            request: super::publication::opened(&published),
            path: Box::new(published.clone()),
        })
        .unwrap();
    same(&inference, &inference);
    let mapped = derivation::compare(&inference, &inference)
        .unwrap()
        .unwrap();
    assert!(
        mapped
            .at(&Address {
                derivation: vec![0, 0],
                state: 1
            })
            .is_some()
    );
    assert_eq!(mapped.branch().len(), 1);
    let restored = super::publication::publish(&erase(&published), {
        let (_, value) = super::publication::wrapper();
        value
    });
    same(&restored, &restored);
}

#[test]
fn anchor() {
    let initial = fixture(0);
    let first = Qualification::new(initial.clone(), world::Identity(0)).unwrap();
    let original = first.inspect(witness(&initial, 0)).unwrap();
    let extended = initial
        .advance(Step::Introduction {
            world: world::Identity(0),
            consumed: vec![],
            value: Construction::new(context::Identity(0), History::default()).literal("Marker"),
        })
        .unwrap();
    let second = Qualification::new(
        extended.clone(),
        extended.target().world().next().unwrap().identity,
    )
    .unwrap();
    let later = second.inspect(witness(&extended, 0)).unwrap();
    let left = publish(&extended, original);
    let right = publish(&extended, later);
    assert_eq!(left.target(), right.target());
    assert_eq!(left.flow(), right.flow());
    assert!(derivation::compare(&left, &right).unwrap().is_none());
}

#[test]
fn unsupported() {
    let (initial, witness) = super::qualification::branch();
    let qualification = Qualification::new(
        initial.clone(),
        initial.target().world().next().unwrap().identity,
    )
    .unwrap();
    let construction = qualification.construction();
    let value = qualification.inspect(witness[0].clone()).unwrap();
    let mut opened = construction
        .open(construction.definition(value).unwrap())
        .unwrap();
    opened.input = construction.input([]).unwrap();
    opened.output = construction.output([]).unwrap();
    let value = construction.close(opened).unwrap();
    let published = publish(&initial, value);
    let world = published.target().world().next().unwrap();
    let code = world
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap();
    let request = Request {
        code: Code::Local {
            world: world.identity,
            occurrence: code.identity,
        },
        selection: vec![Selection {
            world: world.identity,
            occurrence: vec![],
        }],
    };
    let inferred = Path::new(initial.source().clone())
        .advance(Step::Inference {
            path: Box::new(published),
            request,
        })
        .unwrap();
    assert_eq!(inferred.target().frame().count(), 1);
    let result = derivation::compare(&inferred, &inferred);
    assert!(
        matches!(
            result,
            Err(Unsupported {
                side: Side::Left,
                missing: Missing::Context(_),
                ..
            })
        ),
        "{result:?}"
    );
    assert!(matches!(
        derivation::compare(&initial, &inferred),
        Err(Unsupported {
            side: Side::Right,
            ..
        })
    ));
}

fn marker(path: &Path) -> Path {
    path.advance(Step::Introduction {
        world: path.target().world().next().unwrap().identity,
        consumed: vec![],
        value: Construction::new(path.target().root(), History::default()).literal("Marker"),
    })
    .unwrap()
}

#[test]
fn address() {
    let initial = fixture(0);
    let path = marker(&marker(&initial));
    let qualification =
        Qualification::new(path.clone(), path.target().world().next().unwrap().identity).unwrap();
    let left = publish(&path, qualification.inspect(witness(&path, 0)).unwrap());
    let right = publish(&path, qualification.inspect(witness(&path, 1)).unwrap());
    assert_eq!(left.target(), right.target());
    assert_eq!(left.flow(), right.flow());
    assert_eq!(
        left.record().last().unwrap().read,
        right.record().last().unwrap().read
    );
    assert!(derivation::compare(&left, &right).unwrap().is_none());
    let world = initial.target().world().next().unwrap();
    let fragment = Construction::new(context::Identity(0), History::default())
        .inspect(&world.occurrence[0])
        .unwrap();
    let restore = |state| {
        let request = witness(&path, state);
        path.advance(Step::Historical(model::admission::Request {
            world: path.target().world().next().unwrap().identity,
            consumed: vec![],
            value: fragment.clone(),
            witness: vec![model::admission::Witness {
                state,
                world: request.world,
                place: request.place,
            }],
        }))
        .unwrap()
    };
    let left = restore(0);
    let right = restore(1);
    assert_eq!(left.target(), right.target());
    assert_eq!(left.flow(), right.flow());
    assert!(derivation::compare(&left, &right).unwrap().is_none());
}

#[test]
fn ambiguity() {
    let initial = fixture(0);
    let mut world = initial.source().world().cloned().collect::<Vec<_>>();
    world[0].occurrence.push(Occurrence {
        identity: occurrence::Identity(3),
        value: Value::Atom("Call".into()),
        history: History::default(),
    });
    let initial = Path::new(
        Configuration::new(
            initial.source().root(),
            world,
            initial.source().frame().cloned().collect(),
            History::default(),
        )
        .unwrap(),
    );
    let first = request(&initial);
    let mut second = first.clone();
    second.selection[0].occurrence = vec![occurrence::Identity(3)];
    let left = initial.advance(Step::Application(first)).unwrap();
    let right = initial.advance(Step::Application(second)).unwrap();
    same(&left, &right);
    let mapping = derivation::compare(&left, &right).unwrap().unwrap();
    assert_eq!(
        mapping.state()[0].occurrence()[&occurrence::Identity(1)],
        occurrence::Identity(3)
    );
    assert!(
        derivation::find(&left, &right, |mapping| mapping.state()[0].occurrence()
            [&occurrence::Identity(1)]
            == occurrence::Identity(1))
        .unwrap()
        .is_none()
    );
    assert!(
        derivation::find(&left, &right, |_| false)
            .unwrap()
            .is_none()
    );
    assert!(derivation::compare(&initial, &left).unwrap().is_none());
}

#[test]
fn capture() {
    let (_, witness) = super::qualification::branch();
    let (initial, value) = super::publication::wrapper();
    let path = super::publication::publish(&initial, value);
    let qualification =
        Qualification::new(path.clone(), path.target().world().next().unwrap().identity).unwrap();
    let construction = qualification.construction();
    let make = |position: usize| {
        let source = qualification.inspect(witness[position].clone()).unwrap();
        let opened = construction
            .open(construction.definition(source).unwrap())
            .unwrap();
        construction
            .rule(opened.input, construction.output([]).unwrap())
            .unwrap()
    };
    let left = super::publication::publish(&path, make(0));
    let right = super::publication::publish(&path, make(1));
    assert_eq!(left.target(), right.target());
    assert_eq!(left.flow(), right.flow());
    assert!(derivation::compare(&left, &right).unwrap().is_none());
}

#[test]
fn held() {
    let build = |base| {
        let initial = fixture(base);
        let mut world = initial.source().world().cloned().collect::<Vec<_>>();
        world[0].context = context::Identity(base + 1);
        let path = Path::new(
            Configuration::new(
                initial.source().root(),
                world,
                initial.source().frame().cloned().collect(),
                History::default(),
            )
            .unwrap(),
        );
        let qualification = Qualification::new(path.clone(), world::Identity(base)).unwrap();
        let value = qualification
            .inspect(support::Request {
                address: Address::default(),
                world: world::Identity(base),
                place: Place::Held(context::Identity(base + 1), occurrence::Identity(base + 2)),
            })
            .unwrap();
        publish(&path, value)
    };
    let left = build(0);
    let right = build(19);
    same(&left, &right);
    assert_eq!(
        left.record()[0].read,
        std::collections::BTreeSet::from([Place::Held(
            context::Identity(1),
            occurrence::Identity(2)
        )])
    );
    assert!(left.record()[0].consumed.is_empty());
}
