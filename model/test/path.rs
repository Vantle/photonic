use model::admission::{self, Witness};
use model::application::{Code, Request, Selection};
use model::configuration::Configuration;
use model::context::{self, Frame};
use model::failure::Failure;
use model::flow::Place;
use model::history::{Branch, History};
use model::occurrence::Identity;
use model::path::{Path, Step};
use model::structure::Value;
use model::world::{self, World};
use std::collections::BTreeSet;

fn initial(history: History) -> Path {
    let declaration = [("Seed", "A"), ("A", "B"), ("B", "C"), ("C", "D")]
        .into_iter()
        .map(|(input, output)| {
            let Value::Rule(rule) = super::rule(0, input, Value::Atom(output.into())) else {
                unreachable!()
            };
            *rule
        })
        .collect();
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
                declaration,
                held: vec![],
            }],
            history,
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

#[test]
fn inference() {
    let prefix = initial(History::default())
        .advance(Step::Application(request(0, 0, 0)))
        .unwrap();
    let source = prefix.target().clone();
    let support = Path::new(source.clone())
        .advance(Step::Application(Request {
            code: Code::Local {
                world: world::Identity(1),
                occurrence: Identity(1),
            },
            ..request(1, 1, 2)
        }))
        .unwrap();
    let inferred = Path::new(source)
        .advance(Step::Inference {
            path: Box::new(support),
            request: request(2, 2, 3),
        })
        .unwrap();
    let step = Step::Inference {
        path: Box::new(inferred),
        request: request(3, 2, 3),
    };
    let result = prefix.advance(step.clone()).unwrap();
    assert_eq!(prefix.advance(step.clone()).unwrap(), result);
    assert_eq!(
        Path::replay(
            prefix.source().clone(),
            [Step::Application(request(0, 0, 0)), step]
        )
        .unwrap(),
        result
    );
    assert_eq!(prefix.record().len(), 1);
    assert_eq!(result.record().len(), 2);
    let world = result.target().world().next().unwrap();
    assert_eq!(world.occurrence.len(), 2);
    assert!(
        world
            .occurrence
            .iter()
            .any(|value| value.value == Value::Atom("D".into()))
    );
    assert_eq!(
        result.flow().resource[&Place::World(world.identity, Identity(3))],
        BTreeSet::from([Place::World(world::Identity(0), Identity(0))])
    );
    let Step::Inference { path, .. } = &result.record()[1].step else {
        panic!()
    };
    assert!(result.record()[1].read.is_empty());
    assert!(path.record()[0].read.is_empty());
    let Step::Inference { path, .. } = &path.record()[0].step else {
        panic!()
    };
    assert_eq!(
        path.record()[0].read,
        BTreeSet::from([Place::World(world::Identity(1), Identity(1))])
    );
}

#[test]
fn source() {
    let initial = initial(History::default());
    let advanced = initial
        .advance(Step::Application(request(0, 0, 0)))
        .unwrap();
    assert_eq!(
        advanced.advance(Step::Inference {
            path: Box::new(advanced.clone()),
            request: request(1, 1, 2)
        }),
        Err(Failure::Source)
    );
    let different = Configuration::new(
        initial.source().root(),
        initial.source().world().cloned().collect(),
        initial.source().frame().cloned().collect(),
        History::default()
            .decide(model::history::Identity(0), Branch(1))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        initial.advance(Step::Inference {
            path: Box::new(Path::new(different)),
            request: request(0, 0, 0)
        }),
        Err(Failure::Source)
    );
    assert_eq!(
        initial.advance(Step::Inference {
            path: Box::new(initial.clone()),
            request: request(1, 0, 0)
        }),
        Err(Failure::Match(world::Identity(0)))
    );
    assert_eq!(
        initial.advance(Step::Inference {
            path: Box::new(initial.clone()),
            request: request(0, 0, 99)
        }),
        Err(Failure::Occurrence(Identity(99)))
    );
    assert!(initial.record().is_empty());
}

#[test]
fn support() {
    let prefix = initial(History::default())
        .advance(Step::Application(request(0, 0, 0)))
        .unwrap();
    let source = prefix.target();
    let declared = Path::new(source.clone())
        .advance(Step::Application(request(1, 1, 2)))
        .unwrap();
    let local = Path::new(source.clone())
        .advance(Step::Application(Request {
            code: Code::Local {
                world: world::Identity(1),
                occurrence: Identity(1),
            },
            ..request(1, 1, 2)
        }))
        .unwrap();
    assert_eq!(local.target(), declared.target());
    assert_eq!(local.flow(), declared.flow());
    let local = prefix
        .advance(Step::Inference {
            path: Box::new(local),
            request: request(2, 2, 3),
        })
        .unwrap();
    let declared = prefix
        .advance(Step::Inference {
            path: Box::new(declared),
            request: request(2, 2, 3),
        })
        .unwrap();
    assert_eq!(local.target(), declared.target());
    assert_eq!(local.flow(), declared.flow());
    assert_ne!(local, declared);
}

fn construction(path: &Path) -> model::fragment::Fragment<Value> {
    let construction = model::construction::Construction::new(
        context::Identity(0),
        path.source().history().clone(),
    );
    let source = &path.source().world().next().unwrap().occurrence[0];
    super::field(&construction, "A", construction.inspect(source).unwrap())
}

fn restore(value: model::fragment::Fragment<Value>, world: u64) -> admission::Request {
    admission::Request {
        world: world::Identity(world),
        consumed: vec![],
        value,
        witness: vec![Witness {
            state: 0,
            world: world::Identity(0),
            occurrence: Identity(0),
        }],
    }
}

#[test]
fn historical() {
    let initial = initial(History::default());
    let value = construction(&initial);
    let path = initial
        .advance(Step::Application(request(0, 0, 0)))
        .unwrap();
    assert_eq!(
        path.advance(Step::Introduction {
            world: world::Identity(1),
            consumed: vec![],
            value: value.clone()
        }),
        Err(Failure::Occurrence(Identity(0)))
    );
    let step = Step::Historical(restore(value, 1));
    let introduced = path.advance(step.clone()).unwrap();
    assert_eq!(
        Path::replay(
            initial.source().clone(),
            [Step::Application(request(0, 0, 0)), step]
        )
        .unwrap(),
        introduced
    );
    assert!(introduced.record()[1].read.is_empty());
    assert!(introduced.record()[1].consumed.is_empty());
    assert!(introduced.flow().resource[&Place::World(world::Identity(2), Identity(3))].is_empty());
    let result = introduced
        .advance(Step::Application(Request {
            code: Code::Local {
                world: world::Identity(2),
                occurrence: Identity(3),
            },
            ..request(0, 2, 2)
        }))
        .unwrap();
    assert!(
        result
            .target()
            .world()
            .next()
            .unwrap()
            .occurrence
            .iter()
            .any(|value| value.value == Value::Atom("Seed".into()))
    );
    let Step::Historical(request) = &result.record()[1].step else {
        panic!()
    };
    assert_eq!(
        request.witness,
        vec![Witness {
            state: 0,
            world: world::Identity(0),
            occurrence: Identity(0)
        }]
    );
    assert_eq!(
        result.flow().resource[&Place::World(world::Identity(3), Identity(4))],
        BTreeSet::from([Place::World(world::Identity(0), Identity(0))])
    );
}

#[test]
fn current() {
    let path = initial(History::default());
    let value = construction(&path);
    let live = path
        .advance(Step::Introduction {
            world: world::Identity(0),
            consumed: vec![],
            value: value.clone(),
        })
        .unwrap();
    let historical = path.advance(Step::Historical(restore(value, 0))).unwrap();
    assert_eq!(historical.target(), live.target());
    assert_eq!(historical.flow(), live.flow());
    assert_eq!(historical.record()[0].read, live.record()[0].read);
}

#[test]
fn admission() {
    let initial = initial(History::default());
    let value = construction(&initial);
    let path = initial
        .advance(Step::Application(request(0, 0, 0)))
        .unwrap();
    let original = restore(value, 1);
    let mut missing = original.clone();
    missing.witness.clear();
    assert_eq!(
        path.advance(Step::Historical(missing)),
        Err(Failure::Witness(Identity(0)))
    );
    let mut repeated = original.clone();
    repeated.witness.push(repeated.witness[0]);
    assert_eq!(
        path.advance(Step::Historical(repeated)),
        Err(Failure::Witness(Identity(0)))
    );
    let mut extra = original.clone();
    extra.witness.push(Witness {
        occurrence: Identity(1),
        ..extra.witness[0]
    });
    assert_eq!(
        path.advance(Step::Historical(extra)),
        Err(Failure::Witness(Identity(1)))
    );
    let mut future = original.clone();
    future.witness[0].state = 2;
    assert_eq!(
        path.advance(Step::Historical(future)),
        Err(Failure::State(2))
    );
    let mut absent = original.clone();
    absent.witness[0].world = world::Identity(9);
    assert_eq!(
        path.advance(Step::Historical(absent)),
        Err(Failure::World(world::Identity(9)))
    );
    let mut consumed = original.clone();
    consumed.consumed.push(Identity(0));
    assert_eq!(
        path.advance(Step::Historical(consumed)),
        Err(Failure::Occurrence(Identity(0)))
    );
    let construction =
        model::construction::Construction::new(context::Identity(0), History::default());
    let mismatched = super::field(
        &construction,
        "A",
        construction
            .inspect(&super::source(0, Value::Atom("Other".into())))
            .unwrap(),
    );
    assert_eq!(
        path.advance(Step::Historical(admission::Request {
            value: mismatched,
            ..original.clone()
        })),
        Err(Failure::Identity(Identity(0)))
    );
    let construction = model::construction::Construction::new(
        context::Identity(0),
        History::default()
            .decide(model::history::Identity(0), Branch(1))
            .unwrap(),
    );
    let foreign = super::field(
        &construction,
        "A",
        construction
            .inspect(&initial.source().world().next().unwrap().occurrence[0])
            .unwrap(),
    );
    assert!(matches!(
        path.advance(Step::Historical(admission::Request {
            value: foreign,
            ..original
        })),
        Err(Failure::History { .. })
    ));
    assert_eq!(path.record().len(), 1);
}

#[test]
fn lineage() {
    let initial = initial(History::default());
    let value = construction(&initial);
    let advanced = initial
        .advance(Step::Application(request(0, 0, 0)))
        .unwrap()
        .advance(Step::Application(request(1, 1, 2)))
        .unwrap();
    assert!(
        advanced
            .advance(Step::Historical(restore(value, 2)))
            .is_ok()
    );
    let mut world = initial.source().world().cloned().collect::<Vec<_>>();
    world.push(World {
        identity: world::Identity(9),
        context: context::Identity(0),
        occurrence: vec![super::source(9, Value::Atom("Other".into()))],
    });
    let path = Path::new(
        Configuration::new(
            initial.source().root(),
            world,
            initial.source().frame().cloned().collect(),
            initial.source().history().clone(),
        )
        .unwrap(),
    );
    let construction =
        model::construction::Construction::new(context::Identity(0), History::default());
    let value = construction
        .inspect(&path.source().world().nth(1).unwrap().occurrence[0])
        .unwrap();
    let request = admission::Request {
        world: world::Identity(0),
        consumed: vec![],
        value,
        witness: vec![Witness {
            state: 0,
            world: world::Identity(9),
            occurrence: Identity(9),
        }],
    };
    assert_eq!(
        path.advance(Step::Historical(request)),
        Err(Failure::Lineage {
            source: world::Identity(9),
            target: world::Identity(0)
        })
    );
}

#[test]
fn capture() {
    let initial = initial(History::default());
    let mut frame = initial.source().frame().cloned().collect::<Vec<_>>();
    frame.push(Frame {
        identity: context::Identity(1),
        parent: Some(context::Identity(0)),
        lexical: Some(context::Identity(0)),
        declaration: vec![],
        held: vec![],
    });
    let path = Path::new(
        Configuration::new(
            context::Identity(0),
            vec![World {
                identity: world::Identity(0),
                context: context::Identity(0),
                occurrence: vec![super::source(
                    0,
                    super::rule(1, "A", Value::Atom("Private".into())),
                )],
            }],
            frame,
            History::default(),
        )
        .unwrap(),
    );
    let value = construction(&path);
    let construction =
        model::construction::Construction::new(context::Identity(0), History::default());
    let path = path
        .advance(Step::Introduction {
            world: world::Identity(0),
            consumed: vec![Identity(0)],
            value: construction.literal("A"),
        })
        .unwrap();
    assert_eq!(path.target().frame().count(), 1);
    let restored = path
        .advance(Step::Historical(restore(value.clone(), 1)))
        .unwrap();
    assert_eq!(restored.target().frame().count(), 2);
    let archive = &restored.record()[1].archive;
    assert_eq!(archive.len(), 1);
    assert_eq!(archive[&context::Identity(2)].state, 0);
    assert_eq!(archive[&context::Identity(2)].context, context::Identity(1));
    assert!(archive[&context::Identity(2)].resource.is_empty());
    let expected = super::rule(0, "A", super::rule(2, "A", Value::Atom("Private".into())));
    assert!(
        restored
            .target()
            .world()
            .next()
            .unwrap()
            .occurrence
            .iter()
            .any(|value| value.value == expected)
    );
    let repeated = restored
        .advance(Step::Historical(restore(value, 2)))
        .unwrap();
    assert_eq!(repeated.target().frame().count(), 2);
    assert!(repeated.record()[2].archive.is_empty());
    assert_eq!(
        repeated
            .target()
            .world()
            .next()
            .unwrap()
            .occurrence
            .iter()
            .filter(|value| value.value == expected)
            .count(),
        2
    );
}

#[test]
fn mixed() {
    let initial = initial(History::default());
    let path = initial
        .advance(Step::Application(request(0, 0, 0)))
        .unwrap();
    let construction =
        model::construction::Construction::new(context::Identity(0), History::default());
    let past = &path.source().world().next().unwrap().occurrence[0];
    let current = path
        .target()
        .world()
        .next()
        .unwrap()
        .occurrence
        .iter()
        .find(|value| value.identity == Identity(2))
        .unwrap();
    let input = construction
        .input([construction.particle([construction.literal("A")]).unwrap()])
        .unwrap();
    let output = construction
        .output([construction
            .destination(
                construction
                    .particle([
                        construction.inspect(past).unwrap(),
                        construction.inspect(current).unwrap(),
                    ])
                    .unwrap(),
                None,
            )
            .unwrap()])
        .unwrap();
    let value = construction.rule(input, output).unwrap();
    let mut request = restore(value, 1);
    request.witness.push(Witness {
        state: 1,
        world: world::Identity(1),
        occurrence: Identity(2),
    });
    let result = path.advance(Step::Historical(request)).unwrap();
    assert_eq!(
        result.record()[1].read,
        BTreeSet::from([Place::World(world::Identity(1), Identity(2))])
    );
    let Step::Historical(request) = &result.record()[1].step else {
        panic!()
    };
    assert_eq!(request.witness.len(), 2);
    assert!(result.record()[1].consumed.is_empty());
}

#[test]
fn derived() {
    let initial = initial(History::default());
    let support = initial
        .advance(Step::Application(request(0, 0, 0)))
        .unwrap();
    let path = initial
        .advance(Step::Inference {
            path: Box::new(support),
            request: request(1, 1, 2),
        })
        .unwrap();
    let construction =
        model::construction::Construction::new(context::Identity(0), History::default());
    let value = super::field(
        &construction,
        "B",
        construction
            .inspect(&path.source().world().next().unwrap().occurrence[0])
            .unwrap(),
    );
    let path = path.advance(Step::Historical(restore(value, 1))).unwrap();
    let result = path
        .advance(Step::Application(Request {
            code: Code::Local {
                world: world::Identity(2),
                occurrence: Identity(3),
            },
            ..request(0, 2, 2)
        }))
        .unwrap();
    assert_eq!(result.record().len(), 3);
    assert!(matches!(result.record()[0].step, Step::Inference { .. }));
    assert!(
        result
            .target()
            .world()
            .next()
            .unwrap()
            .occurrence
            .iter()
            .any(|value| value.value == Value::Atom("Seed".into()))
    );
    assert_eq!(
        result.flow().resource[&Place::World(world::Identity(3), Identity(4))],
        BTreeSet::from([Place::World(world::Identity(0), Identity(0))])
    );
}
