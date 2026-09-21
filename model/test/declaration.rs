use model::application::{self, Code, Request, Selection};
use model::configuration::Configuration;
use model::construction::Construction;
use model::context::{self, Frame};
use model::environment::Environment;
use model::failure::Failure;
use model::fragment::Fragment;
use model::history::{Branch, History};
use model::machine::Machine;
use model::occurrence::{self, Occurrence};
use model::scope::Scope;
use model::structure::{self, Value};
use model::world::{self, World};
use model::{scope, template};
use std::collections::BTreeSet;
use std::task::Poll;

fn atom(value: &str) -> template::Value {
    template::Value::Atom(value.into())
}

fn rule(input: &str, output: Vec<template::Value>, body: Option<template::Body>) -> template::Rule {
    template::Rule {
        input: template::Input::Build(vec![template::Particle::Build(vec![atom(input)])]),
        output: template::Output::Build(vec![template::Destination {
            particle: template::Particle::Build(output),
            body,
        }]),
    }
}

fn fixture(history: History) -> (Construction, Environment, template::Value) {
    let construction = Construction::new(context::Identity(0), history.clone());
    let origin = Construction::new(context::Identity(7), history.clone());
    let imported = rule("Call", vec![atom("A")], Some(template::Body::Build(vec![])))
        .instantiate(&origin, &Environment::new(scope::Identity(99)))
        .unwrap();
    let occurrence = Occurrence {
        identity: occurrence::Identity(99),
        value: imported.value().clone(),
        history,
    };
    let mut scope = Scope::new(scope::Identity(0));
    let slot = scope.declare().unwrap();
    let initial = Environment::new(scope.identity());
    let environment = Environment {
        value: initial
            .value
            .bind(&slot, construction.inspect(&occurrence).unwrap())
            .unwrap(),
        ..initial
    };
    let local = rule("Call", vec![atom("A")], Some(template::Body::Build(vec![])));
    let maker = rule(
        "Make",
        vec![
            template::Value::Rule(Box::new(local)),
            template::Value::Reference(slot),
        ],
        None,
    );
    let value = template::Value::Rule(Box::new(rule(
        "Enter",
        vec![atom("Make"), atom("Call")],
        Some(template::Body::Build(vec![
            rule("A", vec![atom("Local")], None),
            maker,
        ])),
    )));
    (construction, environment, value)
}

fn step(state: &Configuration, code: Code, input: &str) -> application::Event {
    let world = state.world().next().unwrap();
    let value = world
        .occurrence
        .iter()
        .find(|value| value.value == Value::Atom(input.into()))
        .unwrap();
    application::apply(
        state,
        &Request {
            code,
            selection: vec![Selection {
                world: world.identity,
                occurrence: vec![value.identity],
            }],
        },
    )
    .unwrap()
}

fn declaration(state: &Configuration, context: context::Identity, input: &str) -> Code {
    let frame = state
        .frame()
        .find(|frame| frame.identity == context)
        .unwrap();
    let expected = structure::Input::new(vec![structure::Particle::new(vec![Value::Atom(
        input.into(),
    )])]);
    Code::Declaration {
        context,
        position: frame
            .declaration
            .iter()
            .position(|rule| rule.input == expected)
            .unwrap(),
    }
}

fn execute(value: &Fragment<Value>) -> Vec<Configuration> {
    let history = value.evidence().history().clone();
    let mut frame = vec![Frame {
        identity: context::Identity(0),
        parent: None,
        lexical: None,
        declaration: vec![],
        held: vec![],
    }];
    let foreign = Construction::new(context::Identity(7), value.evidence().history().clone());
    let definition = rule("A", vec![atom("Foreign")], None)
        .instantiate(&foreign, &Environment::new(scope::Identity(99)))
        .unwrap();
    frame.push(Frame {
        identity: context::Identity(7),
        parent: Some(context::Identity(0)),
        lexical: Some(context::Identity(0)),
        declaration: vec![foreign.definition(definition).unwrap().value().clone()],
        held: vec![],
    });
    let state = Configuration::new(
        context::Identity(0),
        vec![World {
            identity: world::Identity(0),
            context: context::Identity(0),
            occurrence: [Value::Atom("Enter".into()), value.value().clone()]
                .into_iter()
                .enumerate()
                .map(|(identity, value)| Occurrence {
                    identity: occurrence::Identity(identity as u64),
                    value,
                    history: history.clone(),
                })
                .collect(),
        }],
        frame,
        value.evidence().history().clone(),
    )
    .unwrap();
    let entered = step(
        &state,
        Code::Local {
            world: world::Identity(0),
            occurrence: occurrence::Identity(1),
        },
        "Enter",
    );
    let owner = entered.target.world().next().unwrap().context;
    let made = step(
        &entered.target,
        declaration(&entered.target, owner, "Make"),
        "Make",
    );
    assert_eq!(
        made.target.world().next().unwrap().context,
        context::Identity(0)
    );
    let mut result = Vec::new();
    for (capture, label) in [(context::Identity(7), "Foreign"), (owner, "Local")] {
        let world = made.target.world().next().unwrap();
        let code = world
            .occurrence
            .iter()
            .find(|value| matches!(&value.value, Value::Rule(rule) if rule.context == capture))
            .unwrap();
        let called = step(
            &made.target,
            Code::Local {
                world: world.identity,
                occurrence: code.identity,
            },
            "Call",
        );
        let done = step(
            &called.target,
            declaration(&called.target, capture, "A"),
            "A",
        );
        assert!(
            done.target
                .world()
                .next()
                .unwrap()
                .occurrence
                .iter()
                .any(|value| value.value == Value::Atom(label.into()))
        );
        result.push(done.target);
    }
    result
}

#[test]
fn suspension() {
    let history = History::default()
        .decide(model::history::Identity(1), Branch(0))
        .unwrap();
    let (construction, environment, value) = fixture(history.clone());
    let expected = value.instantiate(&construction, &environment).unwrap();
    assert_eq!(
        expected.evidence().read(),
        &BTreeSet::from([occurrence::Identity(99)])
    );
    assert_eq!(
        expected.evidence().context(),
        &BTreeSet::from([context::Identity(0), context::Identity(7)])
    );
    assert_eq!(expected.evidence().history(), &history);
    let execution = execute(&expected);
    let mut complete = Machine::new(&value, &construction, &environment);
    assert_eq!(complete.run(100_000), Poll::Ready(Ok(expected.clone())));
    let work = complete.work();
    for boundary in 0..=work {
        let mut machine = Machine::new(&value, &construction, &environment);
        assert_eq!(
            machine.run(boundary),
            if boundary == work {
                Poll::Ready(Ok(expected.clone()))
            } else {
                Poll::Pending
            }
        );
        let Poll::Ready(Ok(result)) = machine.run(work - boundary) else {
            panic!("construction must finish")
        };
        assert_eq!(result, expected);
        assert_eq!(execute(&result), execution);
        assert_eq!(machine.run(0), Poll::Ready(Ok(expected.clone())));
        assert_eq!(machine.work(), work);
    }
}

#[test]
fn failure() {
    let fork = model::history::Identity(1);
    let history = History::default().decide(fork, Branch(0)).unwrap();
    let (_, environment, value) = fixture(history);
    for history in [
        History::default(),
        History::default().decide(fork, Branch(1)).unwrap(),
    ] {
        let construction = Construction::new(context::Identity(0), history);
        assert!(matches!(
            value.instantiate(&construction, &environment),
            Err(Failure::History { .. })
        ));
        super::machine::compare(&value, &construction, &environment);
    }
    let construction = Construction::new(context::Identity(0), History::default());
    let empty = Environment::new(scope::Identity(0));
    assert!(matches!(
        value.instantiate(&construction, &empty),
        Err(Failure::Binding { .. })
    ));
    super::machine::compare(&value, &construction, &empty);
}

#[test]
fn sealing() {
    let construction = Construction::new(context::Identity(0), History::default());
    let deferred = construction.defer();
    let local = deferred.local();
    let value = local
        .rule(local.input([]).unwrap(), local.output([]).unwrap())
        .unwrap();
    assert_eq!(construction.seal(value.clone()), Err(Failure::Depth(0)));
    let body = deferred.body([local.definition(value).unwrap()]).unwrap();
    let sealed = construction.seal(body).unwrap();
    assert_eq!(
        sealed.value().activate(context::Identity(9)).unwrap()[0].context,
        context::Identity(9)
    );
    assert_eq!(
        sealed.evidence().context(),
        &BTreeSet::from([context::Identity(0)])
    );
}

#[test]
fn generator() {
    let (construction, environment, value) = fixture(History::default());
    let expected = value.instantiate(&construction, &environment).unwrap();
    let definition = model::generator::Definition {
        declaration: model::parameter::Declaration::new(scope::Identity(1), vec![]).unwrap(),
        term: model::generator::Term::Value(value),
    };
    let generator =
        model::generator::Generator::close(definition, environment, &construction).unwrap();
    let invocation = generator.bind(&construction, vec![]).unwrap();
    let model::generator::Product::Value(direct) = invocation.evaluate(&construction).unwrap()
    else {
        panic!("expected closed code")
    };
    assert_eq!(direct, expected);
    let mut machine = invocation.machine(&construction).unwrap();
    let result = loop {
        if let Poll::Ready(result) = machine.run(1) {
            break result.unwrap();
        }
    };
    assert_eq!(result, direct);
    assert_eq!(execute(&result), execute(&direct));
}

#[test]
fn sort() {
    let construction = Construction::new(context::Identity(0), History::default());
    let origin = Construction::new(context::Identity(7), History::default());
    let captured = origin
        .inspect(&super::source(
            99,
            super::rule(7, "Call", Value::Atom("Leaf".into())),
        ))
        .unwrap();
    let particle = origin.particle([captured.clone()]).unwrap();
    let input = origin.input([particle.clone()]).unwrap();
    let output = origin
        .output([origin.destination(particle.clone(), None).unwrap()])
        .unwrap();
    let body = origin
        .body([origin.definition(captured.clone()).unwrap()])
        .unwrap();
    let mut scope = Scope::new(scope::Identity(0));
    let value = scope.declare().unwrap();
    let group = scope.declare().unwrap();
    let entry = scope.declare().unwrap();
    let exit = scope.declare().unwrap();
    let nested = scope.declare().unwrap();
    let initial = Environment::new(scope.identity());
    let environment = Environment {
        value: initial.value.bind(&value, captured.clone()).unwrap(),
        particle: initial.particle.bind(&group, particle).unwrap(),
        input: initial.input.bind(&entry, input).unwrap(),
        output: initial.output.bind(&exit, output).unwrap(),
        body: initial.body.bind(&nested, body.clone()).unwrap(),
    };
    let declaration = vec![
        template::Rule {
            input: template::Input::Reference(entry),
            output: template::Output::Reference(exit),
        },
        template::Rule {
            input: template::Input::Build(vec![template::Particle::Reference(group)]),
            output: template::Output::Build(vec![template::Destination {
                particle: template::Particle::Build(vec![template::Value::Reference(value)]),
                body: Some(template::Body::Reference(nested)),
            }]),
        },
    ];
    let template = template::Value::Rule(Box::new(rule(
        "Enter",
        vec![],
        Some(template::Body::Build(declaration)),
    )));
    super::machine::compare(&template, &construction, &environment);
    let result = template.instantiate(&construction, &environment).unwrap();
    assert_eq!(
        result.evidence().read(),
        &BTreeSet::from([occurrence::Identity(99)])
    );
    assert_eq!(
        result.evidence().context(),
        &BTreeSet::from([context::Identity(0), context::Identity(7)])
    );
    let Value::Rule(rule) = result.value() else {
        unreachable!()
    };
    let activated = rule.output.destination()[0]
        .body
        .as_ref()
        .unwrap()
        .activate(context::Identity(9))
        .unwrap();
    for declaration in activated {
        assert_eq!(declaration.context, context::Identity(9));
        assert_eq!(
            &declaration.input.particle()[0].value()[0],
            captured.value()
        );
        let output = &declaration.output.destination()[0];
        assert_eq!(&output.particle.value()[0], captured.value());
        if let Some(retained) = &output.body {
            assert_eq!(retained, body.value());
        }
    }
}

#[test]
fn depth() {
    let construction = Construction::new(context::Identity(0), History::default());
    let environment = Environment::new(scope::Identity(0));
    for depth in [1, 2, 3, 8, 16, 32] {
        let mut declaration = rule("Call", vec![atom("Leaf")], None);
        for _ in 0..depth {
            declaration = rule(
                "Call",
                vec![],
                Some(template::Body::Build(vec![declaration])),
            );
        }
        let template = template::Value::Rule(Box::new(declaration));
        let expected = template.instantiate(&construction, &environment).unwrap();
        let mut machine = Machine::new(&template, &construction, &environment);
        let result = loop {
            if let Poll::Ready(result) = machine.run(1) {
                break result.unwrap();
            }
        };
        assert_eq!(result, expected);
        let Value::Rule(mut rule) = result.value().clone() else {
            unreachable!()
        };
        for identity in 1..=depth {
            let body = rule.output.destination()[0].body.as_ref().unwrap();
            assert_eq!(body.context(), context::Identity(identity - 1));
            *rule = body
                .activate(context::Identity(identity))
                .unwrap()
                .remove(0);
            assert_eq!(rule.context, context::Identity(identity));
        }
        assert_eq!(
            rule.output.destination()[0].particle.value(),
            &[Value::Atom("Leaf".into())]
        );
    }
}
