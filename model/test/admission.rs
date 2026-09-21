use model::admission::{self, Witness};
use model::application::{Code, Request, Selection};
use model::configuration::Configuration;
use model::construction::Construction;
use model::context::{self, Frame};
use model::failure::Failure;
use model::flow::Place;
use model::fragment::Fragment;
use model::history::History;
use model::occurrence::Identity;
use model::path::{Path, Step};
use model::structure::{Body, Destination, Output, Particle, Value};
use model::world::{self, World};
use std::collections::BTreeSet;

fn initial(value: Value) -> Path {
    let Value::Rule(declaration) = super::rule(1, "A", Value::Atom("Local".into())) else {
        unreachable!()
    };
    Path::new(
        Configuration::new(
            context::Identity(0),
            vec![World {
                identity: world::Identity(0),
                context: context::Identity(1),
                occurrence: vec![
                    super::source(1, Value::Atom("A".into())),
                    super::source(2, Value::Atom("Call".into())),
                ],
            }],
            vec![
                Frame {
                    identity: context::Identity(0),
                    parent: None,
                    lexical: None,
                    declaration: vec![],
                    held: vec![],
                },
                Frame {
                    identity: context::Identity(1),
                    parent: Some(context::Identity(0)),
                    lexical: Some(context::Identity(0)),
                    declaration: vec![*declaration],
                    held: vec![super::source(0, value)],
                },
            ],
            History::default(),
        )
        .unwrap(),
    )
}

fn retain(path: &Path) -> Fragment<Value> {
    let frame = path
        .source()
        .frame()
        .find(|frame| frame.identity == context::Identity(1))
        .unwrap();
    Construction::new(context::Identity(0), History::default())
        .inspect(&frame.held[0])
        .unwrap()
}

fn request(path: &Path, value: Fragment<Value>) -> admission::Request {
    admission::Request {
        world: path.target().world().next().unwrap().identity,
        consumed: vec![],
        value,
        witness: vec![Witness {
            state: 0,
            world: world::Identity(0),
            place: Place::Held(context::Identity(1), Identity(0)),
        }],
    }
}

fn returned(path: &Path) -> Path {
    path.advance(Step::Application(Request {
        code: Code::Declaration {
            context: context::Identity(1),
            position: 0,
        },
        selection: vec![Selection {
            world: world::Identity(0),
            occurrence: vec![Identity(1)],
        }],
    }))
    .unwrap()
}

#[test]
fn held() {
    for historical in [false, true] {
        let initial = initial(Value::Atom("Secret".into()));
        let value = retain(&initial);
        let path = if historical {
            returned(&initial)
        } else {
            initial.clone()
        };
        assert_eq!(
            path.target().frame().count(),
            if historical { 1 } else { 2 }
        );
        let request = request(&path, value);
        let mut consumed = request.clone();
        consumed.consumed.push(Identity(0));
        assert_eq!(
            path.advance(Step::Historical(consumed)),
            Err(Failure::Occurrence(Identity(0)))
        );
        let result = path.advance(Step::Historical(request.clone())).unwrap();
        let record = result.record().last().unwrap();
        assert_eq!(record.step, Step::Historical(request));
        assert!(record.consumed.is_empty());
        assert_eq!(
            record.read,
            if historical {
                BTreeSet::new()
            } else {
                BTreeSet::from([Place::Held(context::Identity(1), Identity(0))])
            }
        );
        let world = result.target().world().next().unwrap();
        let value = world
            .occurrence
            .iter()
            .find(|value| value.value == Value::Atom("Secret".into()))
            .unwrap();
        assert_ne!(value.identity, Identity(0));
        assert!(result.flow().resource[&Place::World(world.identity, value.identity)].is_empty());
        assert_eq!(
            result.target().frame().cloned().collect::<Vec<_>>(),
            path.target().frame().cloned().collect::<Vec<_>>()
        );
        assert_eq!(
            Path::replay(
                initial.source().clone(),
                result.record().iter().map(|record| record.step.clone())
            )
            .unwrap(),
            result
        );
    }
}

#[test]
fn mixed() {
    let initial = initial(Value::Atom("Secret".into()));
    let construction = Construction::new(context::Identity(0), History::default());
    let world = initial.source().world().next().unwrap();
    let output = construction
        .output([construction
            .destination(
                construction
                    .particle([
                        retain(&initial),
                        construction.inspect(&world.occurrence[0]).unwrap(),
                    ])
                    .unwrap(),
                None,
            )
            .unwrap()])
        .unwrap();
    let value = construction
        .rule(construction.input([]).unwrap(), output)
        .unwrap();
    let mut request = request(&initial, value);
    request.consumed.push(Identity(1));
    request.witness.push(Witness {
        state: 0,
        world: world::Identity(0),
        place: Place::World(world::Identity(0), Identity(1)),
    });
    let result = initial.advance(Step::Historical(request)).unwrap();
    let world = result.target().world().next().unwrap();
    let code = world
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap();
    let consumed = BTreeSet::from([Place::World(world::Identity(0), Identity(1))]);
    assert_eq!(result.record()[0].consumed, consumed);
    assert_eq!(
        result.flow().resource[&Place::World(world.identity, code.identity)],
        consumed
    );
    assert_eq!(
        result.record()[0].read,
        BTreeSet::from([
            Place::Held(context::Identity(1), Identity(0)),
            Place::World(world::Identity(0), Identity(1)),
        ])
    );
    assert_eq!(
        result.target().frame().nth(1).unwrap().held,
        initial.source().frame().nth(1).unwrap().held
    );
}

#[test]
fn rejection() {
    let initial = initial(Value::Atom("Secret".into()));
    let mut world = initial.source().world().cloned().collect::<Vec<_>>();
    world.push(World {
        identity: world::Identity(9),
        context: context::Identity(0),
        occurrence: vec![super::source(9, Value::Atom("Other".into()))],
    });
    let path = Path::new(
        Configuration::new(
            context::Identity(0),
            world,
            initial.source().frame().cloned().collect(),
            History::default(),
        )
        .unwrap(),
    );
    let original = request(&path, retain(&path));
    let mut owner = original.clone();
    owner.witness[0].world = world::Identity(9);
    assert_eq!(
        path.advance(Step::Historical(owner)),
        Err(Failure::Owner {
            world: world::Identity(9),
            context: context::Identity(1),
        })
    );
    let mut lineage = original.clone();
    lineage.world = world::Identity(9);
    assert_eq!(
        path.advance(Step::Historical(lineage)),
        Err(Failure::Lineage {
            source: world::Identity(0),
            target: world::Identity(9),
        })
    );
    for place in [
        Place::World(world::Identity(0), Identity(0)),
        Place::World(world::Identity(9), Identity(0)),
        Place::Held(context::Identity(9), Identity(0)),
    ] {
        let mut request = original.clone();
        request.witness[0].place = place;
        let expected = match place {
            Place::World(world::Identity(0), _) => Failure::Occurrence(Identity(0)),
            Place::World(identity, _) => Failure::World(identity),
            Place::Held(context, _) => Failure::Owner {
                world: world::Identity(0),
                context,
            },
        };
        assert_eq!(path.advance(Step::Historical(request)), Err(expected));
    }
    let construction = Construction::new(context::Identity(0), History::default());
    for identity in [0, 99] {
        let mut request = original.clone();
        request.value = construction
            .inspect(&super::source(identity, Value::Atom("Changed".into())))
            .unwrap();
        request.witness[0].place = Place::Held(context::Identity(1), Identity(identity));
        assert_eq!(
            path.advance(Step::Historical(request)),
            Err(if identity == 0 {
                Failure::Identity(Identity(0))
            } else {
                Failure::Occurrence(Identity(99))
            })
        );
    }
    let mut repeated = original.clone();
    repeated.witness.push(Witness {
        place: Place::World(world::Identity(0), Identity(0)),
        ..repeated.witness[0]
    });
    assert_eq!(
        path.advance(Step::Historical(repeated)),
        Err(Failure::Witness(Identity(0)))
    );
    assert!(path.advance(Step::Historical(original)).is_ok());
    assert!(path.record().is_empty());
}

#[test]
fn capture() {
    let Value::Rule(mut code) = super::rule(1, "Call", Value::Atom("A".into())) else {
        unreachable!()
    };
    code.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Atom("A".into())]),
        body: Some(Body::new(context::Identity(1), vec![])),
    }]);
    let initial = initial(Value::Rule(code));
    let path = returned(&initial);
    assert_eq!(path.target().frame().count(), 1);
    let restored = path
        .advance(Step::Historical(request(&path, retain(&initial))))
        .unwrap();
    let world = restored.target().world().next().unwrap();
    let code = world
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap();
    let frame = restored.target().frame().nth(1).unwrap();
    assert_eq!(
        restored.flow().frame[&frame.identity],
        Some(context::Identity(1))
    );
    assert_eq!(
        restored.flow().resource[&Place::Held(frame.identity, frame.held[0].identity)],
        BTreeSet::from([Place::Held(context::Identity(1), Identity(0))])
    );
    let construction = Construction::new(context::Identity(0), History::default());
    assert_eq!(
        restored.advance(Step::Historical(admission::Request {
            world: world.identity,
            consumed: vec![],
            value: construction.inspect(&frame.held[0]).unwrap(),
            witness: vec![Witness {
                state: 2,
                world: world.identity,
                place: Place::Held(frame.identity, frame.held[0].identity),
            }],
        })),
        Err(Failure::Owner {
            world: world.identity,
            context: frame.identity,
        })
    );
    let called = restored
        .advance(Step::Application(Request {
            code: Code::Local {
                world: world.identity,
                occurrence: code.identity,
            },
            selection: vec![Selection {
                world: world.identity,
                occurrence: vec![Identity(2)],
            }],
        }))
        .unwrap();
    let world = called.target().world().next().unwrap();
    let argument = world
        .occurrence
        .iter()
        .find(|value| value.value == Value::Atom("A".into()))
        .unwrap();
    let result = called
        .advance(Step::Application(Request {
            code: Code::Declaration {
                context: frame.identity,
                position: 0,
            },
            selection: vec![Selection {
                world: world.identity,
                occurrence: vec![argument.identity],
            }],
        }))
        .unwrap();
    let world = result.target().world().next().unwrap();
    let output = world
        .occurrence
        .iter()
        .max_by_key(|value| value.identity)
        .unwrap();
    assert_eq!(output.value, Value::Atom("Local".into()));
    assert_eq!(
        result.flow().resource[&Place::World(world.identity, output.identity)],
        BTreeSet::from([Place::World(world::Identity(0), Identity(2))])
    );
    assert!(result.record()[1].read.is_empty());
    assert!(result.record()[1].consumed.is_empty());
    assert_eq!(
        Path::replay(
            initial.source().clone(),
            result.record().iter().map(|record| record.step.clone())
        )
        .unwrap(),
        result
    );
}
