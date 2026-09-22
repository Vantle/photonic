use model::application::{Code, Request, Selection};
use model::capture::Capture;
use model::configuration::Configuration;
use model::construction::Construction;
use model::context::{self, Frame};
use model::environment::Environment;
use model::failure::Failure;
use model::flow::Place;
use model::fragment::Fragment;
use model::history::History;
use model::occurrence;
use model::path::{Path, Step};
use model::qualification::Qualification;
use model::reconstruction;
use model::scope::Scope;
use model::structure::{Body, Destination, Input, Output, Particle, Rule, Value};
use model::support::{self, Address};
use model::transition::Transition;
use model::world::{self, World};
use model::{scope, template};
use std::collections::BTreeSet;

fn inspect(source: &Transition) -> (Qualification, Fragment<Value<Capture, Capture>>) {
    let Code::Local { world, occurrence } = source.request().code else {
        panic!()
    };
    let qualification = Qualification::new(source.path().clone(), world).unwrap();
    let value = qualification
        .inspect(support::Request {
            address: Address {
                derivation: vec![],
                state: source.path().record().len(),
            },
            world,
            place: Place::World(world, occurrence),
        })
        .unwrap();
    (qualification, value)
}

fn rebuild(source: &Transition) -> Fragment<Value<Capture, Capture>> {
    let (qualification, value) = inspect(source);
    let construction = qualification.construction();
    let mut opened = construction
        .open(construction.definition(value.clone()).unwrap())
        .unwrap();
    let mut scope = Scope::new(scope::Identity(0));
    let input = scope.declare().unwrap();
    let output = scope.declare().unwrap();
    let environment = Environment::new(scope.identity());
    let environment = Environment {
        input: environment.input.bind(&input, opened.input).unwrap(),
        output: environment.output.bind(&output, opened.output).unwrap(),
        ..environment
    };
    opened.input = template::Input::Reference(input)
        .instantiate(construction, &environment)
        .unwrap();
    opened.output = template::Output::Reference(output)
        .instantiate(construction, &environment)
        .unwrap();
    let rebuilt = construction.close(opened).unwrap();
    assert_eq!(rebuilt, value);
    let slot = scope.declare().unwrap();
    let environment = Environment {
        value: environment.value.bind(&slot, rebuilt.clone()).unwrap(),
        ..environment
    };
    super::machine::compare(
        &template::Value::Reference(slot),
        construction,
        &environment,
    );
    rebuilt
}

fn verify(source: &Transition, target: &Transition) {
    let original = source.path();
    let reconstructed = target.path();
    assert_eq!(reconstructed.record().len(), original.record().len() + 1);
    assert!(reconstructed.state().starts_with(original.state()));
    assert!(reconstructed.record().starts_with(original.record()));
    let record = reconstructed.record().last().unwrap();
    let Step::Construction(request) = &record.step else {
        panic!()
    };
    let Code::Local { world, occurrence } = source.request().code else {
        panic!()
    };
    assert_eq!(request.world, world);
    assert_eq!(request.consumed, vec![occurrence]);
    assert_eq!(
        record.read,
        BTreeSet::from([Place::World(world, occurrence)])
    );
    assert_eq!(record.consumed, record.read);
    assert_eq!(request.value, inspect(source).1);
    assert_eq!(
        original.flow().compose(&record.flow).unwrap(),
        *reconstructed.flow()
    );
    assert_eq!(source.binding(), target.binding());
    assert_eq!(source.event(), target.event());
    let before = source.retain().unwrap();
    let after = target.retain().unwrap();
    assert_eq!(before.target(), after.target());
    assert_eq!(before.flow(), after.flow());
    assert_ne!(before, after);
    assert!(
        model::derivation::compare(&before, &after)
            .unwrap()
            .is_none()
    );
    for path in [original, reconstructed, &before, &after] {
        super::publication::validate(path);
    }
}

fn check(source: &Transition) -> Transition {
    let target = reconstruction::apply(source, rebuild(source)).unwrap();
    verify(source, &target);
    target
}

fn local(path: &Path, label: &str) -> Transition {
    let world = path.target().world().next().unwrap();
    let code = world
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap();
    let value = world
        .occurrence
        .iter()
        .find(|value| value.value == Value::Atom(label.into()))
        .unwrap();
    Transition::new(
        path.clone(),
        Request {
            code: Code::Local {
                world: world.identity,
                occurrence: code.identity,
            },
            selection: vec![Selection {
                world: world.identity,
                occurrence: vec![value.identity],
            }],
        },
    )
    .unwrap()
}

fn fixture(changed: bool, shape: usize, base: u64) -> Transition {
    let label = if changed { "B" } else { "A" };
    let Value::Rule(producer) = super::rule(base, "A", Value::Atom(label.into())) else {
        panic!()
    };
    let output = match shape {
        0 => vec![],
        1 => vec![Destination {
            particle: Particle::default(),
            body: None,
        }],
        2 => vec![Destination {
            particle: Particle::new(vec![Value::Atom("Result".into())]),
            body: Some(Body::new(context::Identity(base), vec![])),
        }],
        3 => vec![
            Destination {
                particle: Particle::new(vec![Value::Atom("Result".into())]),
                body: None,
            },
            Destination {
                particle: Particle::new(vec![Value::Atom("Result".into())]),
                body: Some(Body::new(context::Identity(base), vec![])),
            },
        ],
        _ => unreachable!(),
    };
    let consumer = Rule {
        context: context::Identity(base),
        input: Input::new(vec![Particle::new(vec![Value::Atom(label.into())])]),
        output: Output::new(output),
    };
    let initial = Path::new(
        Configuration::new(
            context::Identity(base),
            vec![World {
                identity: world::Identity(base),
                context: context::Identity(base),
                occurrence: vec![
                    super::source(base, Value::Atom("A".into())),
                    super::source(base + 1, Value::Rule(Box::new(consumer))),
                    super::source(base + 2, Value::Atom("Keep".into())),
                ],
            }],
            vec![Frame {
                identity: context::Identity(base),
                parent: None,
                lexical: None,
                declaration: vec![*producer],
                held: vec![],
            }],
            History::default(),
        )
        .unwrap(),
    );
    let path = initial
        .advance(Step::Application(Request {
            code: Code::Declaration {
                context: context::Identity(base),
                position: 0,
            },
            selection: vec![Selection {
                world: world::Identity(base),
                occurrence: vec![occurrence::Identity(base)],
            }],
        }))
        .unwrap();
    local(&path, label)
}

#[test]
fn matrix() {
    for changed in [false, true] {
        for shape in 0..4 {
            for base in [0, 19] {
                let source = fixture(changed, shape, base);
                assert_eq!(source.binding().footprint.len(), 1);
                assert_eq!(source.binding().exact.len(), usize::from(!changed));
                assert_eq!(source.binding().read.len(), 1);
                let once = check(&source);
                check(&once);
            }
        }
    }
}

#[test]
fn restoration() {
    for shared in [false, true] {
        let original = super::archive::initial(shared);
        check(&local(&original, "Call"));
        let fragment = super::archive::retain(&original, occurrence::Identity(0));
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
            path = super::archive::restore(&erased, fragment.clone(), 0, occurrence::Identity(0));
            check(&local(&path, "Call"));
        }
    }
}

#[test]
fn capture() {
    let (initial, value) = super::publication::wrapper();
    let mut path = super::publication::publish(&initial, value.clone());
    for _ in 0..3 {
        let source = Transition::new(path.clone(), super::publication::opened(&path)).unwrap();
        let rebuilt = check(&source);
        super::publication::execute(&rebuilt.retain().unwrap());
        let Code::Local { world, occurrence } = source.request().code else {
            panic!()
        };
        let erased = path
            .advance(Step::Introduction {
                world,
                consumed: vec![occurrence],
                value: Construction::new(context::Identity(0), History::default())
                    .literal("Bridge"),
            })
            .unwrap();
        path = super::publication::publish(&erased, value.clone());
    }
}

#[test]
fn rejection() {
    let source = fixture(true, 2, 0);
    let (qualification, value) = inspect(&source);
    let construction = qualification.construction();
    let mut opened = construction
        .open(construction.definition(value.clone()).unwrap())
        .unwrap();
    opened.output = construction.output([]).unwrap();
    let changed = construction.close(opened).unwrap();
    assert_eq!(changed.evidence(), value.evidence());
    let Code::Local { world, occurrence } = source.request().code else {
        panic!()
    };
    assert_eq!(
        reconstruction::apply(&source, changed),
        Err(Failure::Match(world))
    );
    let detached = Qualification::new(Path::new(source.path().target().clone()), world).unwrap();
    let other = detached
        .inspect(support::Request {
            address: Address::default(),
            world,
            place: Place::World(world, occurrence),
        })
        .unwrap();
    assert_eq!(other.value(), value.value());
    assert_eq!(
        reconstruction::apply(&source, other),
        Err(Failure::Witness(occurrence))
    );
    let constant = construction
        .rule(
            construction
                .input([construction.particle([construction.literal("B")]).unwrap()])
                .unwrap(),
            construction
                .output([construction
                    .destination(
                        construction
                            .particle([construction.literal("Result")])
                            .unwrap(),
                        Some(construction.body([]).unwrap()),
                    )
                    .unwrap()])
                .unwrap(),
        )
        .unwrap();
    assert_eq!(constant.value(), value.value());
    assert_eq!(
        reconstruction::apply(&source, constant),
        Err(Failure::Witness(occurrence))
    );
    let operand = source
        .path()
        .target()
        .world()
        .next()
        .unwrap()
        .occurrence
        .iter()
        .find(|value| value.value == Value::Atom("B".into()))
        .unwrap()
        .identity;
    let extra = qualification
        .inspect(support::Request {
            address: Address {
                derivation: vec![],
                state: source.path().record().len(),
            },
            world,
            place: Place::World(world, operand),
        })
        .unwrap();
    let mut opened = construction
        .open(construction.definition(value.clone()).unwrap())
        .unwrap();
    opened.input = construction
        .input([construction.particle([extra]).unwrap()])
        .unwrap();
    let extra = construction.close(opened).unwrap();
    assert_eq!(extra.value(), value.value());
    assert_eq!(extra.evidence().qualified().count(), 2);
    assert_eq!(
        reconstruction::apply(&source, extra),
        Err(Failure::Witness(occurrence))
    );
    let current = source.path().target();
    let foreign = Path::new(
        Configuration::new(
            current.root(),
            current.world().cloned().collect(),
            current.frame().cloned().collect(),
            current
                .history()
                .decide(model::history::Identity(0), model::history::Branch(1))
                .unwrap(),
        )
        .unwrap(),
    );
    let qualification = Qualification::new(foreign, world).unwrap();
    let foreign = qualification
        .inspect(support::Request {
            address: Address::default(),
            world,
            place: Place::World(world, occurrence),
        })
        .unwrap();
    assert_eq!(foreign.value(), value.value());
    assert_ne!(foreign.evidence().history(), value.evidence().history());
    assert_eq!(
        reconstruction::apply(&source, foreign),
        Err(Failure::Witness(occurrence))
    );
    let declared = Transition::new(
        Path::new(source.path().source().clone()),
        Request {
            code: Code::Declaration {
                context: context::Identity(0),
                position: 0,
            },
            selection: vec![Selection {
                world: world::Identity(0),
                occurrence: vec![occurrence::Identity(0)],
            }],
        },
    )
    .unwrap();
    assert_eq!(reconstruction::apply(&declared, value), Err(Failure::Rule));
    let previous = source.clone();
    check(&source);
    assert_eq!(source, previous);
}

#[test]
fn sharing() {
    let code = super::source(
        1,
        Value::Rule(Box::new(Rule {
            context: context::Identity(0),
            input: Input::new(vec![
                Particle::default(),
                Particle::new(vec![Value::Atom("A".into())]),
            ]),
            output: Output::new(vec![Destination {
                particle: Particle::new(vec![Value::Atom("Result".into())]),
                body: None,
            }]),
        })),
    );
    let path = Path::new(
        Configuration::new(
            context::Identity(0),
            vec![
                World {
                    identity: world::Identity(0),
                    context: context::Identity(0),
                    occurrence: vec![super::source(0, Value::Atom("A".into())), code.clone()],
                },
                World {
                    identity: world::Identity(1),
                    context: context::Identity(0),
                    occurrence: vec![code],
                },
            ],
            vec![Frame {
                identity: context::Identity(0),
                parent: None,
                lexical: None,
                declaration: vec![],
                held: vec![],
            }],
            History::default(),
        )
        .unwrap(),
    );
    let source = Transition::new(
        path,
        Request {
            code: Code::Local {
                world: world::Identity(0),
                occurrence: occurrence::Identity(1),
            },
            selection: vec![
                Selection {
                    world: world::Identity(1),
                    occurrence: vec![],
                },
                Selection {
                    world: world::Identity(0),
                    occurrence: vec![occurrence::Identity(0)],
                },
            ],
        },
    )
    .unwrap();
    let target = check(&source);
    assert!(model::comparison::compare(source.path().target(), target.path().target()).is_none());
    assert_eq!(target.request().selection[0].world, world::Identity(1));
    assert_eq!(
        source
            .event()
            .target
            .world()
            .next()
            .unwrap()
            .occurrence
            .len(),
        2
    );
}

#[test]
fn read() {
    let source = fixture(true, 2, 0);
    let Code::Local { world, .. } = source.request().code else {
        panic!()
    };
    let path = source
        .path()
        .advance(Step::Construction(model::publication::Request {
            world,
            consumed: vec![],
            value: rebuild(&source),
        }))
        .unwrap();
    let world = path.target().world().next().unwrap();
    let emitted = world.occurrence.last().unwrap().identity;
    let mut request = source.request().clone();
    request.code = Code::Local {
        world: world.identity,
        occurrence: emitted,
    };
    request.selection[0].world = world.identity;
    let target = Transition::new(path, request).unwrap();
    assert_eq!(source.event().target, target.event().target);
    assert_eq!(source.event().flow, target.event().flow);
    assert_eq!(source.event().consumed, target.event().consumed);
    assert_eq!(source.event().read.len(), 1);
    assert!(target.event().read.is_empty());
    assert!(!target.path().record().last().unwrap().read.is_empty());
    assert!(model::transition::boundary(&source, &target).is_none());
    super::publication::validate(target.path());
}

#[test]
fn inference() {
    let original = fixture(false, 3, 0);
    let path = original.path();
    let world = path.target().world().next().unwrap();
    let occurrence = world
        .occurrence
        .iter()
        .find(|value| value.value == Value::Atom("A".into()))
        .unwrap()
        .identity;
    let inferred = Path::new(path.source().clone())
        .advance(Step::Inference {
            path: Box::new(path.clone()),
            request: Request {
                code: Code::Declaration {
                    context: context::Identity(0),
                    position: 0,
                },
                selection: vec![Selection {
                    world: world.identity,
                    occurrence: vec![occurrence],
                }],
            },
        })
        .unwrap();
    let once = local(&inferred, "A");
    let rebuilt = check(&once);
    let first = rebuilt.path().record().first().unwrap();
    assert!(matches!(first.step, Step::Inference { .. }));
    let retained = once.retain().unwrap();
    let Step::Inference { path, .. } = &retained.record()[0].step else {
        panic!()
    };
    assert!(matches!(path.record()[0].step, Step::Inference { .. }));
}

#[test]
fn generated() {
    let code = super::rule(0, "A", Value::Atom("Result".into()));
    let Value::Rule(producer) = super::rule(0, "Seed", code) else {
        panic!()
    };
    let initial = Path::new(
        Configuration::new(
            context::Identity(0),
            vec![World {
                identity: world::Identity(0),
                context: context::Identity(0),
                occurrence: vec![
                    super::source(0, Value::Atom("Seed".into())),
                    super::source(1, Value::Atom("A".into())),
                ],
            }],
            vec![Frame {
                identity: context::Identity(0),
                parent: None,
                lexical: None,
                declaration: vec![*producer],
                held: vec![],
            }],
            History::default(),
        )
        .unwrap(),
    );
    let path = initial
        .advance(Step::Application(Request {
            code: Code::Declaration {
                context: context::Identity(0),
                position: 0,
            },
            selection: vec![Selection {
                world: world::Identity(0),
                occurrence: vec![occurrence::Identity(0)],
            }],
        }))
        .unwrap();
    let source = local(&path, "A");
    check(&source);
    assert_eq!(
        source.binding().read,
        BTreeSet::from([Place::World(world::Identity(0), occurrence::Identity(0))])
    );
    assert_eq!(
        source.binding().footprint,
        BTreeSet::from([Place::World(world::Identity(0), occurrence::Identity(1))])
    );
    let qualification = Qualification::new(initial, world::Identity(0)).unwrap();
    let descriptor = qualification
        .inspect(support::Request {
            address: Address::default(),
            world: world::Identity(0),
            place: Place::World(world::Identity(0), occurrence::Identity(0)),
        })
        .unwrap();
    assert_eq!(
        qualification.construction().definition(descriptor),
        Err(Failure::Rule)
    );
}

#[test]
fn capacity() {
    let path = Path::new(
        Configuration::new(
            context::Identity(0),
            vec![World {
                identity: world::Identity(0),
                context: context::Identity(0),
                occurrence: vec![super::source(
                    u64::MAX,
                    Value::Rule(Box::new(Rule {
                        context: context::Identity(0),
                        input: Input::default(),
                        output: Output::default(),
                    })),
                )],
            }],
            vec![Frame {
                identity: context::Identity(0),
                parent: None,
                lexical: None,
                declaration: vec![],
                held: vec![],
            }],
            History::default(),
        )
        .unwrap(),
    );
    let source = Transition::new(
        path,
        Request {
            code: Code::Local {
                world: world::Identity(0),
                occurrence: occurrence::Identity(u64::MAX),
            },
            selection: vec![Selection {
                world: world::Identity(0),
                occurrence: vec![],
            }],
        },
    )
    .unwrap();
    let original = source.clone();
    assert_eq!(
        reconstruction::apply(&source, rebuild(&source)),
        Err(Failure::Capacity)
    );
    assert_eq!(source, original);
}

#[test]
fn transient() {
    for width in [1, 2, 3] {
        let initial = super::correspondence::initial(0, width, 1);
        let (path, request) = super::correspondence::auxiliary(initial.target(), width);
        let source = Transition::new(path, request).unwrap();
        let target = check(&source);
        assert_eq!(target.event().target.world().count(), 0);
        assert_eq!(target.event().target.frame().count(), 1);
        assert_eq!(target.event().archive.len(), width);
        let copy = target
            .event()
            .archive
            .values()
            .flat_map(|origin| origin.resource.values().copied())
            .collect::<BTreeSet<_>>();
        assert_eq!(copy.len(), 1);
    }
}
