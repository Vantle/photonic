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
