use model::application::{Code, Request, Selection};
use model::capture::Capture;
use model::configuration::Configuration;
use model::context::{self, Frame, Reference};
use model::failure::Failure;
use model::flow::Place;
use model::fragment::Fragment;
use model::history::{Branch, History};
use model::occurrence::Identity;
use model::path::{Path, Step};
use model::qualification::Qualification;
use model::structure::{Body, Destination, Output, Particle, Rule, Value};
use model::support::{self, Address};
use model::world::{self, World};

fn request(derivation: Vec<usize>, state: usize, world: u64, occurrence: u64) -> support::Request {
    support::Request {
        address: Address { derivation, state },
        world: world::Identity(world),
        place: Place::World(world::Identity(world), Identity(occurrence)),
    }
}

fn owner(value: &Fragment<Value<Capture, Capture>>) -> &Capture {
    let Value::Rule(rule) = value.value() else {
        panic!()
    };
    &rule.context
}

#[test]
fn mixed() {
    let path = super::support::captured();
    let qualification = Qualification::new(path.clone(), world::Identity(1)).unwrap();
    let construction = qualification.construction();
    let auxiliary = qualification.inspect(request(vec![0], 2, 2, 3)).unwrap();
    let local = construction
        .rule(
            construction
                .input([construction
                    .particle([construction.literal("Call")])
                    .unwrap()])
                .unwrap(),
            construction
                .output([construction
                    .destination(
                        construction
                            .particle([construction.literal("Private")])
                            .unwrap(),
                        None,
                    )
                    .unwrap()])
                .unwrap(),
        )
        .unwrap();
    assert_eq!(owner(&auxiliary).identity(), owner(&local).identity());
    assert_eq!(owner(&auxiliary).address().derivation, vec![0]);
    assert!(owner(&local).address().derivation.is_empty());
    assert_ne!(auxiliary.value(), local.value());
    let forward = construction
        .particle([auxiliary.clone(), local.clone()])
        .unwrap();
    let backward = construction.particle([local, auxiliary.clone()]).unwrap();
    assert_eq!(forward, backward);
    assert_eq!(forward.value().value().len(), 2);
    assert_eq!(forward.evidence().qualified().count(), 1);
    assert_eq!(forward.evidence().capture().len(), 2);
    assert!(forward.evidence().read().is_empty());
    assert!(forward.evidence().context().is_empty());
    assert_eq!(forward.evidence().proof().unwrap().path(), &path);
    assert_eq!(
        forward.evidence().proof().unwrap().world(),
        world::Identity(1)
    );
    let repeated = construction
        .particle([auxiliary.clone(), auxiliary.clone()])
        .unwrap();
    assert_eq!(repeated.value().value().len(), 2);
    assert_eq!(repeated.evidence().qualified().count(), 1);
    let mut inspection = construction
        .open(construction.definition(auxiliary.clone()).unwrap())
        .unwrap();
    inspection.output = construction.output([]).unwrap();
    let rewritten = construction.close(inspection).unwrap();
    assert_eq!(owner(&rewritten), owner(&auxiliary));
    assert_eq!(rewritten.evidence(), auxiliary.evidence());
}

#[test]
fn binding() {
    let qualification = Qualification::new(super::support::captured(), world::Identity(1)).unwrap();
    let construction = qualification.construction();
    let auxiliary = qualification.inspect(request(vec![0], 2, 2, 3)).unwrap();
    let deferred = construction.defer();
    let local = deferred.local();
    let declaration = local
        .rule(
            local.input([]).unwrap(),
            local
                .output([local
                    .destination(
                        local
                            .particle([deferred.capture(auxiliary.clone()).unwrap()])
                            .unwrap(),
                        None,
                    )
                    .unwrap()])
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        construction.seal(declaration.clone()),
        Err(Failure::Depth(0))
    );
    let body = construction
        .seal(
            deferred
                .body([local.definition(declaration).unwrap()])
                .unwrap(),
        )
        .unwrap();
    assert_eq!(body.value().rule()[0].context, Reference::Local(0));
    let activated = body.value().activate(body.value().context()).unwrap();
    assert_eq!(activated[0].context, body.value().context());
    let Value::Rule(captured) = &activated[0].output.destination()[0].particle.value()[0] else {
        panic!()
    };
    assert_eq!(&captured.context, owner(&auxiliary));
    assert_ne!(captured.context, activated[0].context);
    assert_eq!(body.evidence(), auxiliary.evidence());
    assert_eq!(
        construction
            .seal(deferred.capture(auxiliary.clone()).unwrap())
            .unwrap(),
        auxiliary
    );
}

#[test]
fn held() {
    let qualification = Qualification::new(super::support::captured(), world::Identity(1)).unwrap();
    let mut value = Vec::new();
    for (derivation, occurrence, expected) in [(vec![], 1, "Call"), (vec![0], 0, "Enter")] {
        let request = support::Request {
            address: Address {
                derivation,
                state: 1,
            },
            world: world::Identity(1),
            place: Place::Held(context::Identity(1), Identity(occurrence)),
        };
        let fragment = qualification.inspect(request.clone()).unwrap();
        assert_eq!(fragment.value(), &Value::Atom(expected.into()));
        assert_eq!(fragment.evidence().qualified().next().unwrap().0, &request);
        value.push(fragment);
    }
    let combined = qualification.construction().particle(value).unwrap();
    assert_eq!(combined.evidence().qualified().count(), 2);
    assert!(combined.evidence().read().is_empty());
    assert_eq!(
        qualification.inspect(support::Request {
            address: Address {
                derivation: vec![0],
                state: 2
            },
            world: world::Identity(2),
            place: Place::Held(context::Identity(1), Identity(0)),
        }),
        Err(Failure::Owner {
            world: world::Identity(2),
            context: context::Identity(1)
        })
    );
}

#[test]
fn inheritance() {
    let path = super::support::derivation();
    let qualification = Qualification::new(path, world::Identity(2)).unwrap();
    let mut retained = Vec::new();
    for location in [
        request(vec![], 0, 0, 1),
        request(vec![0], 1, 1, 1),
        request(vec![0, 0], 1, 1, 1),
    ] {
        retained.push(qualification.inspect(location).unwrap());
    }
    assert!(
        retained
            .windows(2)
            .all(|pair| pair[0].value() == pair[1].value())
    );
    for value in &retained {
        assert_eq!(owner(value).address(), &Address::default());
        assert_eq!(owner(value).identity(), context::Identity(0));
    }
    let combined = qualification.construction().particle(retained).unwrap();
    assert_eq!(combined.evidence().capture().len(), 1);
    assert_eq!(combined.evidence().qualified().count(), 3);
}

#[test]
fn archive() {
    let initial = super::archive::initial(false);
    let value = super::archive::retain(&initial, Identity(0));
    let erased = super::archive::remove(&initial, vec![Identity(0)]);
    let path = super::archive::restore(&erased, value, 0, Identity(0));
    let world = path.target().world().next().unwrap();
    let code = world
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap();
    let current = request(vec![], 2, world.identity.0, code.identity.0);
    let qualification = Qualification::new(path.clone(), world.identity).unwrap();
    let original = qualification.inspect(request(vec![], 0, 0, 0)).unwrap();
    let restored = qualification.inspect(current).unwrap();
    assert_eq!(original.value(), restored.value());
    assert_eq!(owner(&restored).identity(), context::Identity(1));
    assert_eq!(owner(&restored).address(), &Address::default());
    assert_ne!(original.evidence(), restored.evidence());
    let combined = qualification
        .construction()
        .particle([original, restored])
        .unwrap();
    assert_eq!(combined.evidence().capture().len(), 2);
    assert_eq!(combined.evidence().qualified().count(), 2);
}

fn enter(label: &str) -> Rule {
    let Value::Rule(closure) = super::rule(0, "Call", Value::Atom("Secret".into())) else {
        unreachable!()
    };
    let mut closure = model::activation::capture(*closure);
    closure.context = Reference::Local(0);
    closure.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Atom("Secret".into())]),
        body: Some(Body::nested(Reference::Local(0), vec![])),
    }]);
    let Value::Rule(producer) = super::rule(0, "Make", Value::Atom("Unused".into())) else {
        unreachable!()
    };
    let mut producer = model::activation::capture(*producer);
    producer.context = Reference::Local(0);
    producer.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Rule(Box::new(closure))]),
        body: None,
    }]);
    let Value::Rule(secret) = super::rule(0, "Secret", Value::Atom(label.into())) else {
        unreachable!()
    };
    let mut secret = model::activation::capture(*secret);
    secret.context = Reference::Local(0);
    let Value::Rule(mut enter) = super::rule(0, "Enter", Value::Atom("Make".into())) else {
        unreachable!()
    };
    enter.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Atom("Make".into())]),
        body: Some(Body::bind(context::Identity(0), vec![producer, secret]).unwrap()),
    }]);
    *enter
}

fn step(path: &Path, context: u64, position: usize, label: &str) -> Request {
    let world = path.target().world().next().unwrap();
    let value = world
        .occurrence
        .iter()
        .find(|value| value.value == Value::Atom(label.into()))
        .unwrap();
    Request {
        code: Code::Declaration {
            context: context::Identity(context),
            position,
        },
        selection: vec![Selection {
            world: world.identity,
            occurrence: vec![value.identity],
        }],
    }
}

#[test]
fn sibling() {
    let Value::Rule(tick) = super::rule(0, "Tick", Value::Atom("Tick".into())) else {
        unreachable!()
    };
    let mut path = Path::new(
        Configuration::new(
            context::Identity(0),
            vec![World {
                identity: world::Identity(0),
                context: context::Identity(0),
                occurrence: ["Enter", "Call", "Tick"]
                    .into_iter()
                    .enumerate()
                    .map(|(identity, label)| {
                        super::source(identity as u64, Value::Atom(label.into()))
                    })
                    .collect(),
            }],
            vec![Frame {
                identity: context::Identity(0),
                parent: None,
                lexical: None,
                declaration: vec![enter("Left"), enter("Right"), *tick],
                held: vec![],
            }],
            History::default(),
        )
        .unwrap(),
    );
    let mut witness = Vec::new();
    for position in 0..2 {
        let auxiliary = Path::new(path.target().clone());
        let auxiliary = auxiliary
            .advance(Step::Application(step(&auxiliary, 0, position, "Enter")))
            .unwrap();
        let auxiliary = auxiliary
            .advance(Step::Application(step(&auxiliary, 1, 0, "Make")))
            .unwrap();
        let world = auxiliary.target().world().next().unwrap();
        let code = world
            .occurrence
            .iter()
            .find(|value| matches!(value.value, Value::Rule(_)))
            .unwrap();
        witness.push(request(
            vec![position],
            2,
            world.identity.0,
            code.identity.0,
        ));
        let request = step(&auxiliary, 0, 2, "Tick");
        path = path
            .advance(Step::Inference {
                path: Box::new(auxiliary),
                request,
            })
            .unwrap();
    }
    let qualification = Qualification::new(path, world::Identity(2)).unwrap();
    let left = qualification.inspect(witness[0].clone()).unwrap();
    let right = qualification.inspect(witness[1].clone()).unwrap();
    assert_eq!(
        left.evidence().qualified().next().unwrap().1.value,
        right.evidence().qualified().next().unwrap().1.value
    );
    assert_eq!(owner(&left).identity(), owner(&right).identity());
    assert_ne!(owner(&left), owner(&right));
    assert_eq!(owner(&left).address().derivation, vec![0]);
    assert_eq!(owner(&right).address().derivation, vec![1]);
    let current = qualification.inspect(request(vec![], 2, 2, 4)).unwrap();
    assert_eq!(witness[0].place.occurrence(), Identity(4));
    let combined = qualification
        .construction()
        .particle([left, right, current])
        .unwrap();
    assert_eq!(combined.evidence().qualified().count(), 3);
    assert_eq!(combined.evidence().capture().len(), 3);
    assert_eq!(combined.value().value().len(), 3);
}

#[test]
fn proof() {
    let path = super::support::captured();
    let original = Qualification::new(path.clone(), world::Identity(1)).unwrap();
    let same = Qualification::new(path.clone(), world::Identity(1)).unwrap();
    let detached =
        Qualification::new(Path::new(path.target().clone()), world::Identity(1)).unwrap();
    assert!(
        original
            .construction()
            .particle([same.construction().literal("A")])
            .is_ok()
    );
    assert_eq!(
        original
            .construction()
            .particle([detached.construction().literal("A")]),
        Err(Failure::Source)
    );
    let history = History::default()
        .decide(model::history::Identity(0), Branch(1))
        .unwrap();
    let state = Configuration::new(
        path.target().root(),
        path.target().world().cloned().collect(),
        path.target().frame().cloned().collect(),
        history,
    )
    .unwrap();
    let foreign = Qualification::new(Path::new(state), world::Identity(1)).unwrap();
    assert!(matches!(
        original
            .construction()
            .particle([foreign.construction().literal("A")]),
        Err(Failure::History { .. })
    ));
    assert_eq!(
        original.inspect(request(vec![9], 0, 0, 0)),
        Err(Failure::Derivation {
            depth: 0,
            position: 9
        })
    );
}
