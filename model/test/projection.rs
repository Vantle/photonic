use model::application::{Code, Request, Selection};
use model::configuration::Configuration;
use model::context::{self, Frame};
use model::failure::Failure;
use model::flow::Place;
use model::history::History;
use model::occurrence::{Identity, Occurrence};
use model::path::{Path, Step};
use model::projection;
use model::structure::{Input, Output, Particle, Rule, Value};
use model::world::{self, World};
use std::collections::BTreeSet;

fn rule(input: &str, output: &str) -> Rule {
    let Value::Rule(rule) = super::rule(0, input, Value::Atom(output.into())) else {
        unreachable!()
    };
    *rule
}

fn frame(identity: u64, parent: Option<u64>, declaration: Vec<Rule>) -> Frame {
    Frame {
        identity: context::Identity(identity),
        parent: parent.map(context::Identity),
        lexical: parent.map(context::Identity),
        declaration,
        held: vec![],
    }
}

fn state(occurrence: Vec<Occurrence>, declaration: Vec<Rule>) -> Configuration {
    Configuration::new(
        context::Identity(0),
        vec![World {
            identity: world::Identity(0),
            context: context::Identity(0),
            occurrence,
        }],
        vec![frame(0, None, declaration)],
        History::default(),
    )
    .unwrap()
}

fn request(context: u64, position: usize, world: u64, occurrence: &[u64]) -> Request {
    Request {
        code: Code::Declaration {
            context: context::Identity(context),
            position,
        },
        selection: vec![Selection {
            world: world::Identity(world),
            occurrence: occurrence.iter().copied().map(Identity).collect(),
        }],
    }
}

fn place(world: u64, occurrence: u64) -> Place {
    Place::World(world::Identity(world), Identity(occurrence))
}

#[test]
fn replay() {
    let initial = state(
        vec![super::source(0, Value::Atom("A".into()))],
        vec![rule("A", "B"), rule("B", "C")],
    );
    let first = Step::Application(request(0, 0, 0, &[0]));
    let second = Step::Application(request(0, 1, 1, &[1]));
    let path = Path::new(initial.clone()).advance(first.clone()).unwrap();
    let result = path.advance(second.clone()).unwrap();
    assert_eq!(result, Path::replay(initial, [first, second]).unwrap());
    assert_eq!(path.record().len(), 1);
    assert_eq!(result.state().len(), 3);
    assert_eq!(
        result.flow().resource[&place(2, 2)],
        BTreeSet::from([place(0, 0)])
    );
    assert_eq!(
        path.advance(Step::Application(request(0, 1, 1, &[9]))),
        Err(Failure::Occurrence(Identity(9)))
    );
    let empty = state(vec![], vec![rule("A", "A")]);
    assert_eq!(
        Path::replay(empty, [Step::Application(request(0, 0, 0, &[0]))]),
        Err(Failure::Occurrence(Identity(0)))
    );
}

#[test]
fn exact() {
    for label in ["A", "B"] {
        let initial = state(
            vec![super::source(0, Value::Atom("A".into()))],
            vec![rule("A", label), rule(label, "D")],
        );
        let path = Path::new(initial)
            .advance(Step::Application(request(0, 0, 0, &[0])))
            .unwrap();
        let projected = projection::project(&path, request(0, 1, 1, &[1])).unwrap();
        assert_eq!(projected.binding().frame, context::Identity(0));
        assert_eq!(projected.binding().owner, Some(context::Identity(0)));
        assert_eq!(
            projected.binding().world,
            BTreeSet::from([world::Identity(0)])
        );
        assert_eq!(projected.binding().footprint, BTreeSet::from([place(0, 0)]));
        assert_eq!(
            projected.binding().exact,
            if label == "A" {
                BTreeSet::from([place(0, 0)])
            } else {
                BTreeSet::new()
            }
        );
        assert_eq!(projected.path(), &path);
        assert_eq!(projected.request(), &request(0, 1, 1, &[1]));
    }
}

#[test]
fn support() {
    let code = Value::Rule(Box::new(rule("A", "B")));
    let initial = state(
        vec![
            super::source(0, Value::Atom("A".into())),
            super::source(1, code),
        ],
        vec![rule("A", "B"), rule("B", "D")],
    );
    let declared = Path::new(initial.clone())
        .advance(Step::Application(request(0, 0, 0, &[0])))
        .unwrap();
    let local = Path::new(initial)
        .advance(Step::Application(Request {
            code: Code::Local {
                world: world::Identity(0),
                occurrence: Identity(1),
            },
            ..request(0, 0, 0, &[0])
        }))
        .unwrap();
    assert_eq!(local.target(), declared.target());
    assert_eq!(local.flow(), declared.flow());
    assert_ne!(local, declared);
    assert_eq!(local.record()[0].read, BTreeSet::from([place(0, 1)]));
    let projected = projection::project(&local, request(0, 1, 1, &[2])).unwrap();
    assert!(projected.binding().read.is_empty());
    assert_eq!(
        projected.path().record()[0].read,
        BTreeSet::from([place(0, 1)])
    );
}

#[test]
fn introduction() {
    let value = super::source(0, Value::Atom("A".into()));
    let construction =
        model::construction::Construction::new(context::Identity(0), History::default());
    let initial = state(vec![value.clone()], vec![rule("A", "D")]);
    let path = Path::new(initial)
        .advance(Step::Introduction {
            world: world::Identity(0),
            consumed: vec![Identity(0)],
            value: construction.inspect(&value).unwrap(),
        })
        .unwrap();
    let projected = projection::project(&path, request(0, 0, 1, &[1])).unwrap();
    assert_eq!(projected.binding().exact, BTreeSet::from([place(0, 0)]));
    assert_eq!(path.record()[0].read, BTreeSet::from([place(0, 0)]));
}

#[test]
fn captured() {
    for capture in [1, 2] {
        let original = super::rule(1, "Call", Value::Atom("Result".into()));
        let emitted = super::rule(capture, "Call", Value::Atom("Result".into()));
        let mut producer = rule("A", "B");
        producer.input = Input::new(vec![Particle::new(vec![original.clone()])]);
        producer.output = Output::new(vec![model::structure::Destination {
            particle: Particle::new(vec![emitted.clone()]),
            body: None,
        }]);
        let mut consumer = rule("B", "D");
        consumer.input = Input::new(vec![Particle::new(vec![emitted])]);
        let initial = Configuration::new(
            context::Identity(0),
            vec![World {
                identity: world::Identity(0),
                context: context::Identity(0),
                occurrence: vec![super::source(0, original)],
            }],
            vec![
                frame(0, None, vec![producer, consumer]),
                frame(1, Some(0), vec![]),
                frame(2, Some(0), vec![]),
            ],
            History::default(),
        )
        .unwrap();
        let path = Path::new(initial)
            .advance(Step::Application(request(0, 0, 0, &[0])))
            .unwrap();
        let projected = projection::project(&path, request(0, 1, 1, &[1])).unwrap();
        assert_eq!(projected.binding().footprint, BTreeSet::from([place(0, 0)]));
        assert_eq!(projected.binding().exact.is_empty(), capture == 2);
    }
}

#[test]
fn held() {
    let mut child = frame(1, Some(0), vec![rule("A", "B")]);
    child
        .held
        .push(super::source(0, Value::Atom("Held".into())));
    let initial = Configuration::new(
        context::Identity(0),
        vec![World {
            identity: world::Identity(0),
            context: context::Identity(1),
            occurrence: vec![super::source(1, Value::Atom("A".into()))],
        }],
        vec![frame(0, None, vec![rule("B", "D")]), child],
        History::default(),
    )
    .unwrap();
    let path = Path::new(initial)
        .advance(Step::Application(request(1, 0, 0, &[1])))
        .unwrap();
    assert_eq!(
        projection::project(&path, request(0, 0, 1, &[2])).unwrap_err(),
        Failure::Held(Place::Held(context::Identity(1), Identity(0)))
    );
}

#[test]
fn context() {
    let mut producer = rule("A", "B");
    producer.output = Output::new(vec![model::structure::Destination {
        particle: Particle::new(vec![Value::Atom("B".into())]),
        body: Some(model::structure::Body::new(context::Identity(0), vec![])),
    }]);
    let initial = state(
        vec![super::source(0, Value::Atom("A".into()))],
        vec![producer, rule("B", "D")],
    );
    let path = Path::new(initial)
        .advance(Step::Application(request(0, 0, 0, &[0])))
        .unwrap();
    assert_eq!(
        projection::project(&path, request(0, 1, 1, &[1])).unwrap_err(),
        Failure::Origin(context::Identity(1))
    );
    let initial = Configuration::new(
        context::Identity(0),
        vec![World {
            identity: world::Identity(0),
            context: context::Identity(1),
            occurrence: vec![super::source(0, Value::Atom("A".into()))],
        }],
        vec![
            frame(0, None, vec![rule("B", "D")]),
            frame(1, Some(0), vec![rule("A", "B")]),
        ],
        History::default(),
    )
    .unwrap();
    let path = Path::new(initial)
        .advance(Step::Application(request(1, 0, 0, &[0])))
        .unwrap();
    assert_eq!(
        projection::project(&path, request(0, 0, 1, &[1])).unwrap_err(),
        Failure::Context(context::Identity(1))
    );
}

#[test]
fn empty() {
    let mut consumer = rule("A", "D");
    consumer.input = Input::default();
    let path = Path::new(state(vec![], vec![consumer]));
    let projected = projection::project(&path, request(0, 0, 0, &[])).unwrap();
    assert!(projected.binding().footprint.is_empty());
    assert!(projected.binding().exact.is_empty());
    assert_eq!(
        projected.binding().world,
        BTreeSet::from([world::Identity(0)])
    );
    assert!(
        projection::project(
            &path,
            Request {
                selection: vec![],
                ..request(0, 0, 0, &[])
            }
        )
        .is_err()
    );
}

#[test]
fn rewrite() {
    for label in ["A", "B"] {
        let mut consumer = rule(label, "D");
        consumer.output = Output::new(vec![
            model::structure::Destination {
                particle: Particle::new(vec![Value::Atom("Direct".into())]),
                body: None,
            },
            model::structure::Destination {
                particle: Particle::new(vec![Value::Atom("Scoped".into())]),
                body: Some(model::structure::Body::new(context::Identity(0), vec![])),
            },
        ]);
        let initial = state(
            vec![
                super::source(0, Value::Atom("A".into())),
                super::source(1, Value::Atom("Keep".into())),
            ],
            vec![rule("A", label), consumer],
        );
        let path = Path::new(initial.clone())
            .advance(Step::Application(request(0, 0, 0, &[0])))
            .unwrap();
        let projection = projection::project(&path, request(0, 1, 1, &[2])).unwrap();
        let result = projection.apply().unwrap();
        assert_eq!(projection.apply().unwrap(), result);
        assert_eq!(path.source(), &initial);
        for world in result.target.world() {
            let scoped = world.context != context::Identity(0);
            let expected = if scoped && label == "B" {
                vec!["A", "Keep", "Scoped"]
            } else if scoped {
                vec!["Keep", "Scoped"]
            } else {
                vec!["Direct", "Keep"]
            };
            let mut actual = world
                .occurrence
                .iter()
                .map(|value| match &value.value {
                    Value::Atom(label) => label.as_str(),
                    _ => panic!(),
                })
                .collect::<Vec<_>>();
            actual.sort();
            assert_eq!(actual, expected);
            for value in &world.occurrence {
                let origin = if value.value == Value::Atom("Keep".into()) {
                    place(0, 1)
                } else {
                    place(0, 0)
                };
                assert_eq!(
                    result.flow.resource[&Place::World(world.identity, value.identity)],
                    BTreeSet::from([origin])
                );
            }
        }
        let frame = result
            .target
            .frame()
            .find(|frame| frame.identity != context::Identity(0))
            .unwrap();
        assert_eq!(frame.held.len(), usize::from(label == "A"));
        assert_eq!(frame.parent, Some(context::Identity(0)));
        assert_eq!(frame.lexical, Some(context::Identity(0)));
        if label == "A" {
            assert_eq!(frame.held[0], super::source(0, Value::Atom("A".into())));
            assert_eq!(
                result.flow.resource[&Place::Held(frame.identity, Identity(0))],
                BTreeSet::from([place(0, 0)])
            );
        }
        Configuration::new(
            result.target.root(),
            result.target.world().cloned().collect(),
            result.target.frame().cloned().collect(),
            result.target.history().clone(),
        )
        .unwrap();
    }
}

fn escape(regenerate: bool) -> Path {
    use model::context::Reference;
    use model::structure::{Body, Destination};
    let mut generator = model::activation::capture(rule("Make", "Unused"));
    generator.context = Reference::Local(0);
    let mut produced = model::activation::capture(rule("Call", "Unused"));
    produced.context = Reference::Local(0);
    let mut payload = model::activation::capture(rule("Payload", "Result"));
    payload.context = Reference::Local(0);
    produced.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Rule(Box::new(payload))]),
        body: None,
    }]);
    generator.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Rule(Box::new(produced))]),
        body: None,
    }]);
    let mut enter = rule("Enter", "Make");
    enter.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Atom("Make".into())]),
        body: Some(Body::bind(context::Identity(0), vec![generator]).unwrap()),
    }]);
    let initial = state(
        vec![
            super::source(
                0,
                Value::Atom(if regenerate { "Seed" } else { "Enter" }.into()),
            ),
            super::source(1, Value::Atom("Call".into())),
        ],
        vec![enter, rule("Seed", "Enter")],
    );
    let path = Path::new(initial);
    let path = if regenerate {
        path.advance(Step::Application(request(0, 1, 0, &[0])))
            .unwrap()
    } else {
        path
    };
    let world = path.target().world().next().unwrap();
    let entered = world
        .occurrence
        .iter()
        .find(|value| value.value == Value::Atom("Enter".into()))
        .unwrap();
    let step = request(0, 0, world.identity.0, &[entered.identity.0]);
    let path = path.advance(Step::Application(step)).unwrap();
    let world = path.target().world().next().unwrap();
    let made = world
        .occurrence
        .iter()
        .find(|value| value.value == Value::Atom("Make".into()))
        .unwrap();
    let step = request(1, 0, world.identity.0, &[made.identity.0]);
    path.advance(Step::Application(step)).unwrap()
}

#[test]
fn import() {
    for regenerate in [false, true] {
        for mixed in [false, true] {
            let path = escape(regenerate);
            let world = path.target().world().next().unwrap();
            let captured = world
                .occurrence
                .iter()
                .find(|value| matches!(value.value, Value::Rule(_)))
                .unwrap();
            let path = if mixed {
                let construction = model::construction::Construction::new(
                    context::Identity(0),
                    History::default(),
                );
                let value = super::field(
                    &construction,
                    "Call",
                    construction.inspect(captured).unwrap(),
                );
                path.advance(Step::Introduction {
                    world: world.identity,
                    consumed: vec![],
                    value,
                })
                .unwrap()
            } else {
                path
            };
            let world = path.target().world().next().unwrap();
            let code = world
                .occurrence
                .iter()
                .filter(|value| matches!(value.value, Value::Rule(_)))
                .max_by_key(|value| value.identity)
                .unwrap();
            let step = Request {
                code: Code::Local {
                    world: world.identity,
                    occurrence: code.identity,
                },
                selection: vec![Selection {
                    world: world.identity,
                    occurrence: vec![Identity(1)],
                }],
            };
            let projection = projection::project(&path, step).unwrap();
            assert_eq!(
                projection.binding().owner,
                if mixed {
                    Some(context::Identity(0))
                } else {
                    None
                }
            );
            let applied = projection.apply().unwrap();
            let captured = applied
                .target
                .frame()
                .find(|frame| frame.identity != context::Identity(0))
                .unwrap();
            assert_eq!(captured.identity, context::Identity(1));
            assert_eq!(captured.parent, Some(context::Identity(0)));
            assert_eq!(captured.lexical, Some(context::Identity(0)));
            assert_eq!(captured.held.len(), 1);
            assert_eq!(captured.held[0].value, Value::Atom("Enter".into()));
            assert_eq!(captured.held[0].identity == Identity(0), !regenerate);
            assert_eq!(
                applied.flow.resource[&Place::Held(captured.identity, captured.held[0].identity)],
                BTreeSet::from([place(0, 0)])
            );
            assert_eq!(applied.flow.frame[&captured.identity], None);
            let world = applied.target.world().next().unwrap();
            assert_eq!(world.occurrence.len(), 2);
            assert!(
                world
                    .occurrence
                    .iter()
                    .any(|value| value.identity == Identity(0)
                        && value.value
                            == Value::Atom(if regenerate { "Seed" } else { "Enter" }.into()))
            );
            let code = world
                .occurrence
                .iter()
                .find(|value| matches!(value.value, Value::Rule(_)))
                .unwrap();
            let expected = if mixed {
                super::rule(
                    1,
                    "Call",
                    super::rule(1, "Payload", Value::Atom("Result".into())),
                )
            } else {
                super::rule(1, "Payload", Value::Atom("Result".into()))
            };
            assert_eq!(code.value, expected);
            assert_eq!(
                applied.flow.resource[&Place::World(world.identity, code.identity)],
                BTreeSet::from([place(0, 1)])
            );
            Configuration::new(
                applied.target.root(),
                applied.target.world().cloned().collect(),
                applied.target.frame().cloned().collect(),
                applied.target.history().clone(),
            )
            .unwrap();
        }
    }
}
