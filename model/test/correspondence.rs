use model::application::{Code, Request, Selection};
use model::configuration::Configuration;
use model::context::{self, Frame, Reference};
use model::derivation;
use model::flow::Place;
use model::history::History;
use model::occurrence;
use model::path::{Path, Step};
use model::provenance;
use model::qualification::Qualification;
use model::structure::{Body, Destination, Input, Output, Particle, Rule, Value};
use model::support::{self, Address};
use model::world::{self, World};
use std::collections::BTreeSet;

fn initial(base: u64, width: usize, count: u64) -> Path {
    let Value::Rule(seed) = super::rule(base, "Start", Value::Atom("Seed".into())) else {
        unreachable!()
    };
    let closure = Rule {
        context: Reference::Local(0),
        input: Input::default(),
        output: Output::default(),
    };
    let producer = Rule {
        context: Reference::Local(0),
        input: Input::new(vec![Particle::new(vec![Value::Atom("Make".into())])]),
        output: Output::new(vec![Destination {
            particle: Particle::new(vec![Value::Rule(Box::new(closure))]),
            body: None,
        }]),
    };
    let enter = Rule {
        context: context::Identity(base),
        input: Input::new(vec![Particle::new(vec![Value::Atom("Seed".into())])]),
        output: Output::new(vec![
            Destination {
                particle: Particle::new(vec![Value::Atom("Make".into())]),
                body: Some(Body::bind(context::Identity(base), vec![producer]).unwrap()),
            };
            width
        ]),
    };
    let join = Rule {
        context: context::Identity(base),
        input: Input::new(vec![Particle::default(); width]),
        output: Output::new(vec![Destination {
            particle: Particle::default(),
            body: None,
        }]),
    };
    Path::new(
        Configuration::new(
            context::Identity(base),
            (0..count)
                .map(|position| World {
                    identity: world::Identity(base + position),
                    context: context::Identity(base),
                    occurrence: vec![super::source(base + position, Value::Atom("Start".into()))],
                })
                .collect(),
            vec![Frame {
                identity: context::Identity(base),
                parent: None,
                lexical: None,
                declaration: vec![*seed, enter, join],
                held: vec![],
            }],
            History::default(),
        )
        .unwrap(),
    )
}

fn apply(path: &Path, context: context::Identity, position: usize, label: &str) -> Path {
    let (world, occurrence) = path
        .target()
        .world()
        .find_map(|world| {
            world
                .occurrence
                .iter()
                .find(|value| value.value == Value::Atom(label.into()))
                .map(|value| (world.identity, value.identity))
        })
        .unwrap();
    path.advance(Step::Application(Request {
        code: Code::Declaration { context, position },
        selection: vec![Selection {
            world,
            occurrence: vec![occurrence],
        }],
    }))
    .unwrap()
}

fn auxiliary(initial: &Configuration, width: usize) -> (Path, Request) {
    let root = initial.root();
    let path = apply(&Path::new(initial.clone()), root, 0, "Start");
    let mut path = apply(&path, root, 1, "Seed");
    for _ in 0..width {
        let context = path
            .target()
            .world()
            .find(|world| world.context != root)
            .unwrap()
            .context;
        path = apply(&path, context, 0, "Make");
    }
    let selection = path
        .target()
        .world()
        .filter(|world| {
            world
                .occurrence
                .iter()
                .any(|value| matches!(value.value, Value::Rule(_)))
        })
        .map(|world| Selection {
            world: world.identity,
            occurrence: vec![],
        })
        .collect();
    path = path
        .advance(Step::Application(Request {
            code: Code::Declaration {
                context: root,
                position: 2,
            },
            selection,
        }))
        .unwrap();
    let world = path
        .target()
        .world()
        .find(|world| {
            world
                .occurrence
                .iter()
                .any(|value| matches!(value.value, Value::Rule(_)))
        })
        .unwrap();
    let qualification = Qualification::new(path.clone(), world.identity).unwrap();
    let construction = qualification.construction();
    let value = world
        .occurrence
        .iter()
        .map(|value| {
            qualification
                .inspect(support::Request {
                    address: Address {
                        derivation: vec![],
                        state: path.record().len(),
                    },
                    world: world.identity,
                    place: Place::World(world.identity, value.identity),
                })
                .unwrap()
        })
        .collect::<Vec<_>>();
    let mut opened = construction
        .open(construction.definition(value[0].clone()).unwrap())
        .unwrap();
    opened.input = construction
        .input([construction.particle(value[1..].iter().cloned()).unwrap()])
        .unwrap();
    let selected = world.occurrence[1..]
        .iter()
        .map(|value| value.identity)
        .collect();
    let world = world.identity;
    path = path
        .advance(Step::Construction(model::publication::Request {
            world,
            consumed: vec![],
            value: construction.close(opened).unwrap(),
        }))
        .unwrap();
    let world = path
        .target()
        .world()
        .find(|world| {
            world
                .occurrence
                .iter()
                .any(|value| matches!(value.value, Value::Rule(_)))
        })
        .unwrap();
    let request = Request {
        code: Code::Local {
            world: world.identity,
            occurrence: world.occurrence.last().unwrap().identity,
        },
        selection: vec![Selection {
            world: world.identity,
            occurrence: selected,
        }],
    };
    (path, request)
}

fn fixture(base: u64, width: usize, count: u64) -> Path {
    let mut path = initial(base, width, count);
    for _ in 0..count {
        let (support, request) = auxiliary(path.target(), width);
        path = path
            .advance(Step::Inference {
                path: Box::new(support),
                request,
            })
            .unwrap();
    }
    path
}

fn validate(left: &Path, right: &Path) {
    super::publication::validate(left);
    super::publication::validate(right);
    assert!(derivation::compare(left, right).unwrap().is_some());
    assert!(derivation::compare(right, left).unwrap().is_some());
}

#[test]
fn transient() {
    for width in [1, 2, 3] {
        for count in [1, 2] {
            let left = fixture(0, width, count);
            let right = fixture(19, width, count);
            validate(&left, &right);
            let mapping = derivation::compare(&left, &right).unwrap().unwrap();
            assert_eq!(mapping.identity().frame().len(), 1 + width * count as usize);
            assert_eq!(mapping.identity().occurrence().len(), 2 * count as usize);
            assert!(mapping.state().iter().all(|state| state.frame().len() == 1));
            let mut resource = BTreeSet::new();
            for record in left.record() {
                assert_eq!(record.archive.len(), width);
                let copy = record
                    .archive
                    .values()
                    .flat_map(|origin| origin.resource.values().copied())
                    .collect::<BTreeSet<_>>();
                assert_eq!(copy.len(), 1);
                let copy = *copy.first().unwrap();
                assert!(resource.insert(copy));
                assert!(
                    mapping
                        .state()
                        .iter()
                        .all(|state| !state.occurrence().contains_key(&copy))
                );
                assert!(mapping.identity().occurrence().contains_key(&copy));
                assert!(
                    derivation::find(&left, &right, |mapping| mapping.identity().occurrence()
                        [&copy]
                        == occurrence::Identity(19))
                    .unwrap()
                    .is_none()
                );
            }
            assert_eq!(left.target().world().count(), 0);
            assert!(
                derivation::find(&left, &right, |_| false)
                    .unwrap()
                    .is_none()
            );
        }
    }
}

#[test]
fn origin() {
    let left = fixture(0, 2, 2);
    let right = fixture(19, 2, 2);
    let mapping = derivation::compare(&left, &right).unwrap().unwrap();
    for (identity, original) in &left.record()[0].archive {
        let target = mapping.identity().frame()[identity];
        let destination = &right.record()[0].archive[&target];
        let source = mapping.at(&original.address).unwrap();
        let check =
            |destination| provenance::origin(source, mapping.identity(), original, destination);
        assert!(check(destination));
        let mut altered = destination.clone();
        altered.address.derivation.push(0);
        assert!(!check(&altered));
        let mut altered = destination.clone();
        altered.address.state -= 1;
        assert!(!check(&altered));
        let mut altered = destination.clone();
        altered.context = right.source().root();
        assert!(!check(&altered));
        let mut altered = destination.clone();
        altered.resource.clear();
        assert!(!check(&altered));
        let mut altered = destination.clone();
        *altered.resource.values_mut().next().unwrap() = occurrence::Identity(19);
        assert!(!check(&altered));
        let mut altered = destination.clone();
        let copy = *altered.resource.values().next().unwrap();
        altered.resource.clear();
        altered.resource.insert(occurrence::Identity(19), copy);
        assert!(!check(&altered));
        let other = *right.record()[1]
            .archive
            .values()
            .next()
            .unwrap()
            .resource
            .values()
            .next()
            .unwrap();
        let mut altered = destination.clone();
        *altered.resource.values_mut().next().unwrap() = other;
        assert!(!check(&altered));
        let copy = *original.resource.values().next().unwrap();
        assert!(
            derivation::find(&left, &right, |mapping| {
                mapping.identity().occurrence()[&copy] == other
            })
            .unwrap()
            .is_none()
        );
    }
}

fn nested(base: u64) -> Path {
    let initial = initial(base, 2, 2);
    let (support, request) = auxiliary(initial.target(), 2);
    let path = initial
        .advance(Step::Inference {
            path: Box::new(support),
            request,
        })
        .unwrap();
    let world = path.target().world().next().unwrap();
    let request = Request {
        code: Code::Declaration {
            context: initial.source().root(),
            position: 0,
        },
        selection: vec![Selection {
            world: world.identity,
            occurrence: vec![world.occurrence[0].identity],
        }],
    };
    initial
        .advance(Step::Inference {
            path: Box::new(path),
            request,
        })
        .unwrap()
}

#[test]
fn namespace() {
    let left = nested(0);
    let right = nested(19);
    validate(&left, &right);
    let mapping = derivation::compare(&left, &right).unwrap().unwrap();
    assert_eq!(mapping.identity().frame().len(), 1);
    assert_eq!(mapping.branch()[&0].identity().frame().len(), 3);
    assert!(
        mapping
            .at(&Address {
                derivation: vec![0, 0],
                state: 2,
            })
            .is_some()
    );
    let fresh = occurrence::Identity(2);
    assert_eq!(
        mapping.identity().occurrence()[&fresh],
        occurrence::Identity(21)
    );
    assert_eq!(
        mapping.branch()[&0].identity().occurrence()[&fresh],
        occurrence::Identity(21)
    );
    assert!(
        mapping.branch()[&0]
            .state()
            .iter()
            .all(|state| !state.occurrence().contains_key(&fresh))
    );
}
