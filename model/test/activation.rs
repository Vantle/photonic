use model::activation::capture;
use model::context::{Identity, Reference};
use model::failure::Failure;
use model::structure::{Body, Destination, Input, Output, Particle, Rule, Value};

fn rule<Context: Ord>(context: Context) -> Rule<Context> {
    Rule {
        context,
        input: Input::new(vec![Particle::new(vec![Value::Atom("Call".into())])]),
        output: Output::new(vec![Destination {
            particle: Particle::new(vec![Value::Atom("Result".into())]),
            body: None,
        }]),
    }
}

fn emitted(rule: &Rule) -> Vec<Identity> {
    rule.output.destination()[0]
        .particle
        .value()
        .iter()
        .map(|value| {
            let Value::Rule(rule) = value else {
                panic!("expected code")
            };
            rule.context
        })
        .collect()
}

#[test]
fn local() {
    let mut declaration = rule(Reference::Local(0));
    declaration.output = Output::new(vec![Destination {
        particle: Particle::new(vec![
            Value::Rule(Box::new(rule(Reference::Local(0)))),
            Value::Rule(Box::new(capture(rule(Identity(7))))),
        ]),
        body: None,
    }]);
    let body = Body::bind(Identity(0), vec![declaration]).unwrap();
    for identity in [7, 8, 91] {
        let result = body.activate(Identity(identity)).unwrap();
        assert_eq!(result[0].context, Identity(identity));
        let mut expected = vec![Identity(7), Identity(identity)];
        expected.sort();
        assert_eq!(emitted(&result[0]), expected);
    }
    assert_eq!(body.rule()[0].context, Reference::Local(0));
}

#[test]
fn nested() {
    let mut inner = rule(Reference::Local(0));
    inner.output = Output::new(vec![Destination {
        particle: Particle::new(vec![
            Value::Rule(Box::new(rule(Reference::Local(0)))),
            Value::Rule(Box::new(rule(Reference::Local(1)))),
        ]),
        body: None,
    }]);
    let mut outer = rule(Reference::Local(0));
    outer.output = Output::new(vec![Destination {
        particle: Particle::default(),
        body: Some(Body::nested(Reference::Local(0), vec![inner])),
    }]);
    let body = Body::bind(Identity(0), vec![outer]).unwrap();
    for (parent, child) in [(1, 2), (20, 10)] {
        let declaration = body.activate(Identity(parent)).unwrap();
        let nested = declaration[0].output.destination()[0]
            .body
            .as_ref()
            .unwrap();
        assert_eq!(nested.context(), Identity(parent));
        assert_eq!(nested.rule()[0].context, Reference::Local(0));
        let result = nested.activate(Identity(child)).unwrap();
        assert_eq!(result[0].context, Identity(child));
        let mut expected = vec![Identity(parent), Identity(child)];
        expected.sort();
        assert_eq!(emitted(&result[0]), expected);
    }
}

#[test]
fn imported() {
    let imported = Body::bind(Identity(7), vec![rule(Reference::Local(0))]).unwrap();
    let mut closed = rule(Identity(7));
    closed.output = Output::new(vec![Destination {
        particle: Particle::default(),
        body: Some(imported.clone()),
    }]);
    let body = Body::bind(Identity(0), vec![capture(closed)]).unwrap();
    let declaration = body.activate(Identity(99)).unwrap();
    assert_eq!(declaration[0].context, Identity(7));
    assert_eq!(declaration[0].output.destination()[0].body, Some(imported));
    let nested = declaration[0].output.destination()[0]
        .body
        .as_ref()
        .unwrap();
    assert_eq!(
        nested.activate(Identity(100)).unwrap()[0].context,
        Identity(100)
    );
}

#[test]
fn invalid() {
    for depth in [1, 2, usize::MAX] {
        assert_eq!(
            Body::bind(Identity(0), vec![rule(Reference::Local(depth))]),
            Err(Failure::Depth(depth))
        );
        let mut outer = rule(Reference::Local(0));
        outer.input = Input::new(vec![Particle::new(vec![Value::Rule(Box::new(rule(
            Reference::Local(depth),
        )))])]);
        assert_eq!(
            Body::bind(Identity(0), vec![outer]),
            Err(Failure::Depth(depth))
        );
    }
    let mut outer = rule(Reference::Local(0));
    outer.output = Output::new(vec![Destination {
        particle: Particle::default(),
        body: Some(Body::nested(Reference::Local(1), vec![])),
    }]);
    assert_eq!(
        Body::bind(Identity(0), vec![outer.clone()]),
        Err(Failure::Depth(1))
    );
    outer.output = Output::new(vec![Destination {
        particle: Particle::default(),
        body: Some(Body::nested(
            Reference::Local(0),
            vec![rule(Reference::Local(2))],
        )),
    }]);
    assert_eq!(Body::bind(Identity(0), vec![outer]), Err(Failure::Depth(2)));
}

#[test]
fn depth() {
    for depth in [1, 2, 3, 8, 16, 32] {
        let mut declaration = rule(Reference::Local(0));
        declaration.output = Output::new(vec![Destination {
            particle: Particle::new(
                (0..depth)
                    .map(|position| Value::Rule(Box::new(rule(Reference::Local(position)))))
                    .collect(),
            ),
            body: None,
        }]);
        for _ in 1..depth {
            let mut parent = rule(Reference::Local(0));
            parent.output = Output::new(vec![Destination {
                particle: Particle::default(),
                body: Some(Body::nested(Reference::Local(0), vec![declaration])),
            }]);
            declaration = parent;
        }
        let mut body = Body::bind(Identity(0), vec![declaration]).unwrap();
        for level in 1..=depth {
            let mut result = body.activate(Identity(level as u64)).unwrap();
            let declaration = result.remove(0);
            assert_eq!(declaration.context, Identity(level as u64));
            let imported = Body::bind(Identity(99), vec![capture(declaration.clone())]).unwrap();
            assert_eq!(
                imported.activate(Identity(100)).unwrap(),
                vec![declaration.clone()]
            );
            if level == depth {
                assert_eq!(
                    emitted(&declaration),
                    (1..=depth as u64).map(Identity).collect::<Vec<_>>()
                );
                break;
            }
            body = declaration.output.destination()[0].body.clone().unwrap();
            assert_eq!(body.context(), Identity(level as u64));
        }
    }
}

#[test]
fn evidence() {
    let mut declaration = rule(Reference::Local(0));
    declaration.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Rule(Box::new(capture(rule(Identity(7)))))]),
        body: None,
    }]);
    let body = Body::bind(Identity(3), vec![declaration]).unwrap();
    let mut value = rule(Identity(2));
    value.output = Output::new(vec![Destination {
        particle: Particle::default(),
        body: Some(body),
    }]);
    let occurrence = model::occurrence::Occurrence {
        identity: model::occurrence::Identity(4),
        value: Value::Rule(Box::new(value)),
        history: model::history::History::default(),
    };
    let construction =
        model::construction::Construction::new(Identity(99), model::history::History::default());
    let fragment = construction.inspect(&occurrence).unwrap();
    assert_eq!(
        fragment.evidence().context(),
        &[2, 3, 7, 99].into_iter().map(Identity).collect()
    );
    let definition = construction.definition(fragment).unwrap();
    let inspection = construction.open(definition).unwrap();
    assert_eq!(
        construction.close(inspection).unwrap().value(),
        &occurrence.value
    );
}

#[test]
fn execution() {
    use model::application::{Code, Request, Selection, apply};
    use model::configuration::Configuration;
    use model::context::Frame;
    use model::history::History;
    use model::occurrence::{self, Occurrence};
    use model::world::{self, World};

    let mut imported = rule(Identity(7));
    imported.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Atom("A".into())]),
        body: Some(Body::new(Identity(7), vec![])),
    }]);
    let mut local = rule(Reference::Local(0));
    local.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Atom("A".into())]),
        body: Some(Body::nested(Reference::Local(0), vec![])),
    }]);
    let mut maker = rule(Reference::Local(0));
    maker.input = Input::new(vec![Particle::new(vec![Value::Atom("Make".into())])]);
    maker.output = Output::new(vec![Destination {
        particle: Particle::new(vec![
            Value::Rule(Box::new(capture(imported))),
            Value::Rule(Box::new(local)),
        ]),
        body: None,
    }]);
    let mut local = rule(Reference::Local(0));
    local.input = Input::new(vec![Particle::new(vec![Value::Atom("A".into())])]);
    local.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Atom("Local".into())]),
        body: None,
    }]);
    let declaration = Body::bind(Identity(0), vec![maker, local])
        .unwrap()
        .activate(Identity(8))
        .unwrap();
    let position = declaration
        .iter()
        .position(|rule| {
            rule.input == Input::new(vec![Particle::new(vec![Value::Atom("Make".into())])])
        })
        .unwrap();
    let mut foreign = rule(Identity(7));
    foreign.input = Input::new(vec![Particle::new(vec![Value::Atom("A".into())])]);
    foreign.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Atom("Foreign".into())]),
        body: None,
    }]);
    let state = Configuration::new(
        Identity(0),
        vec![World {
            identity: world::Identity(0),
            context: Identity(8),
            occurrence: ["Make", "Call"]
                .into_iter()
                .enumerate()
                .map(|(identity, value)| Occurrence {
                    identity: occurrence::Identity(identity as u64),
                    value: Value::Atom(value.into()),
                    history: History::default(),
                })
                .collect(),
        }],
        vec![
            Frame {
                identity: Identity(0),
                parent: None,
                lexical: None,
                declaration: vec![],
                held: vec![],
            },
            Frame {
                identity: Identity(7),
                parent: Some(Identity(0)),
                lexical: Some(Identity(0)),
                declaration: vec![foreign],
                held: vec![],
            },
            Frame {
                identity: Identity(8),
                parent: Some(Identity(0)),
                lexical: Some(Identity(0)),
                declaration,
                held: vec![],
            },
        ],
        History::default(),
    )
    .unwrap();
    let made = apply(
        &state,
        &Request {
            code: Code::Declaration {
                context: Identity(8),
                position,
            },
            selection: vec![Selection {
                world: world::Identity(0),
                occurrence: vec![occurrence::Identity(0)],
            }],
        },
    )
    .unwrap();
    assert_eq!(made.target.world().next().unwrap().context, Identity(0));
    for (capture, expected) in [(7, "Foreign"), (8, "Local")] {
        let world = made.target.world().next().unwrap();
        let code = world.occurrence.iter().find(|value| matches!(&value.value, Value::Rule(rule) if rule.context == Identity(capture))).unwrap();
        let entered = apply(
            &made.target,
            &Request {
                code: Code::Local {
                    world: world.identity,
                    occurrence: code.identity,
                },
                selection: vec![Selection {
                    world: world.identity,
                    occurrence: vec![occurrence::Identity(1)],
                }],
            },
        )
        .unwrap();
        let world = entered.target.world().next().unwrap();
        assert_eq!(
            entered
                .target
                .frame()
                .find(|frame| frame.identity == world.context)
                .unwrap()
                .lexical,
            Some(Identity(capture))
        );
        let argument = world
            .occurrence
            .iter()
            .find(|value| value.value == Value::Atom("A".into()))
            .unwrap();
        let frame = entered
            .target
            .frame()
            .find(|frame| frame.identity == Identity(capture))
            .unwrap();
        let position = frame
            .declaration
            .iter()
            .position(|rule| {
                rule.input == Input::new(vec![Particle::new(vec![Value::Atom("A".into())])])
            })
            .unwrap();
        let result = apply(
            &entered.target,
            &Request {
                code: Code::Declaration {
                    context: frame.identity,
                    position,
                },
                selection: vec![Selection {
                    world: world.identity,
                    occurrence: vec![argument.identity],
                }],
            },
        )
        .unwrap();
        assert!(
            result
                .target
                .world()
                .next()
                .unwrap()
                .occurrence
                .iter()
                .any(|value| value.value == Value::Atom(expected.into()))
        );
    }
}
