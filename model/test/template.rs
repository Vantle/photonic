use model::construction::Construction;
use model::environment::Environment;
use model::failure::Failure;
use model::history::{Branch, History};
use model::occurrence::{Identity, Occurrence};
use model::scope::Scope;
use model::{context, scope, structure, template};
use std::collections::BTreeSet;

#[test]
fn field() {
    let construction = Construction::new(context::Identity(0), History::default());
    let role = super::source(1, structure::Value::Atom("Unseen".into()));
    let payload = super::source(
        2,
        super::rule(7, "Call", structure::Value::Atom("X".into())),
    );
    for identity in [0, 1, 7, 91] {
        let mut scope = Scope::new(scope::Identity(identity));
        let input = scope.declare().unwrap();
        let output = scope.declare().unwrap();
        let initial = Environment::new(scope.identity());
        let environment = Environment {
            value: initial
                .value
                .bind(&input, construction.inspect(&role).unwrap())
                .unwrap()
                .bind(&output, construction.inspect(&payload).unwrap())
                .unwrap(),
            ..initial
        };
        let value = template::Rule {
            input: template::Input::Build(vec![template::Particle::Build(vec![
                template::Value::Reference(input),
            ])]),
            output: template::Output::Build(vec![template::Destination {
                particle: template::Particle::Build(vec![template::Value::Reference(output)]),
                body: None,
            }]),
        };
        let result = value.instantiate(&construction, &environment).unwrap();
        assert_eq!(
            result.value(),
            &super::rule(0, "Unseen", payload.value.clone())
        );
        assert_eq!(
            result.evidence().read(),
            &BTreeSet::from([Identity(1), Identity(2)])
        );
        assert_eq!(
            result.evidence().context(),
            &BTreeSet::from([context::Identity(0), context::Identity(7)])
        );
    }
}

#[test]
fn lexical() {
    let construction = Construction::new(context::Identity(0), History::default());
    let mut outer = Scope::new(scope::Identity(1));
    let mut inner = Scope::new(scope::Identity(2));
    let first = outer.declare().unwrap();
    let second = inner.declare().unwrap();
    let initial = Environment::new(outer.identity());
    assert_eq!(
        initial.value.resolve(&first),
        Err(Failure::Binding {
            scope: outer.identity(),
            position: 0,
        })
    );
    let parent = Environment {
        value: initial
            .value
            .bind(&first, construction.literal("Parent"))
            .unwrap(),
        ..initial
    };
    let child = parent.nested(inner.identity()).unwrap();
    let bound = Environment {
        value: child
            .value
            .bind(&second, construction.literal("Child"))
            .unwrap(),
        ..child
    };
    assert_eq!(
        bound.value.resolve(&first).unwrap().value(),
        &structure::Value::Atom("Parent".into())
    );
    assert_eq!(
        bound.value.resolve(&second).unwrap().value(),
        &structure::Value::Atom("Child".into())
    );
    assert_eq!(
        parent.value.resolve(&second),
        Err(Failure::Scope(inner.identity()))
    );
    assert_eq!(
        bound.value.bind(&first, construction.literal("Wrong")),
        Err(Failure::Scope(outer.identity()))
    );
    assert_eq!(
        bound.value.bind(&second, construction.literal("Wrong")),
        Err(Failure::Occupied {
            scope: inner.identity(),
            position: 0,
        })
    );
    assert_eq!(
        bound.nested(outer.identity()),
        Err(Failure::Scope(outer.identity()))
    );
    let literal = template::Value::Atom("Parent".into())
        .instantiate(&construction, &bound)
        .unwrap();
    assert_eq!(literal.value(), &structure::Value::Atom("Parent".into()));
    assert!(literal.evidence().read().is_empty());
}

#[test]
fn missing() {
    let construction = Construction::new(context::Identity(0), History::default());
    let mut scope = Scope::new(scope::Identity(0));
    let slot = scope.declare().unwrap();
    let environment = Environment::new(scope.identity());
    let value = template::Value::Rule(Box::new(template::Rule {
        input: template::Input::Build(vec![]),
        output: template::Output::Build(vec![template::Destination {
            particle: template::Particle::Build(vec![template::Value::Reference(slot)]),
            body: None,
        }]),
    }));
    assert_eq!(
        value.instantiate(&construction, &environment),
        Err(Failure::Binding {
            scope: scope.identity(),
            position: 0,
        })
    );
}

#[test]
fn shape() {
    let construction = Construction::new(context::Identity(0), History::default());
    let mut scope = Scope::new(scope::Identity(0));
    let input = scope.declare().unwrap();
    let output = scope.declare().unwrap();
    let particle = scope.declare().unwrap();
    let body = scope.declare().unwrap();
    let original = super::source(1, super::rule(7, "A", structure::Value::Atom("B".into())));
    let inspection = construction
        .open(
            construction
                .definition(construction.inspect(&original).unwrap())
                .unwrap(),
        )
        .unwrap();
    let initial = Environment::new(scope.identity());
    let environment = Environment {
        input: initial.input.bind(&input, inspection.input).unwrap(),
        output: initial.output.bind(&output, inspection.output).unwrap(),
        particle: initial
            .particle
            .bind(&particle, construction.particle([]).unwrap())
            .unwrap(),
        body: initial
            .body
            .bind(&body, construction.body([]).unwrap())
            .unwrap(),
        ..initial
    };
    let value = template::Rule {
        input: template::Input::Reference(input),
        output: template::Output::Reference(output),
    }
    .instantiate(&construction, &environment)
    .unwrap();
    assert_eq!(
        value.value(),
        &super::rule(0, "A", structure::Value::Atom("B".into()))
    );
    assert_eq!(value.evidence().read(), &BTreeSet::from([Identity(1)]));
    let destination = template::Destination {
        particle: template::Particle::Reference(particle),
        body: Some(template::Body::Reference(body)),
    }
    .instantiate(&construction, &environment)
    .unwrap();
    assert_eq!(destination.value().particle, structure::Particle::default());
    assert_eq!(
        destination.value().body.as_ref().unwrap().context(),
        context::Identity(0)
    );
}

#[test]
fn body() {
    let origin = Construction::new(context::Identity(7), History::default());
    let construction = Construction::new(context::Identity(0), History::default());
    let mut scope = Scope::new(scope::Identity(0));
    let body = scope.declare().unwrap();
    let initial = Environment::new(scope.identity());
    let environment = Environment {
        body: initial.body.bind(&body, origin.body([]).unwrap()).unwrap(),
        ..initial
    };
    let retained = template::Body::Reference(body)
        .instantiate(&construction, &environment)
        .unwrap();
    assert_eq!(retained.value().context(), context::Identity(7));
    let built = template::Body::Build(vec![template::Rule {
        input: template::Input::Build(vec![]),
        output: template::Output::Build(vec![]),
    }])
    .instantiate(&construction, &environment)
    .unwrap();
    assert_eq!(built.value().context(), context::Identity(0));
    assert_eq!(built.value().rule().len(), 1);
    assert_eq!(built.value().rule()[0].context, context::Identity(0));
}

#[test]
fn history() {
    let fork = model::history::Identity(1);
    let history = History::default().decide(fork, Branch(0)).unwrap();
    let origin = Construction::new(context::Identity(0), history.clone());
    let construction = Construction::new(context::Identity(0), History::default());
    let mut scope = Scope::new(scope::Identity(0));
    let slot = scope.declare().unwrap();
    let occurrence = Occurrence {
        identity: Identity(1),
        value: structure::Value::Atom("Payload".into()),
        history,
    };
    let initial = Environment::new(scope.identity());
    let environment = Environment {
        value: initial
            .value
            .bind(&slot, origin.inspect(&occurrence).unwrap())
            .unwrap(),
        ..initial
    };
    let value = template::Particle::Build(vec![
        template::Value::Reference(slot.clone()),
        template::Value::Reference(slot),
    ]);
    assert!(value.instantiate(&construction, &environment).is_err());
    let result = value.instantiate(&origin, &environment).unwrap();
    assert_eq!(result.value().value().len(), 2);
    assert_eq!(result.evidence().read(), &BTreeSet::from([Identity(1)]));
}

#[test]
fn growth() {
    let construction = Construction::new(context::Identity(0), History::default());
    let mut scope = Scope::new(scope::Identity(0));
    let slot = scope.declare().unwrap();
    let template = template::Rule {
        input: template::Input::Build(vec![template::Particle::Build(vec![
            template::Value::Atom("Step".into()),
        ])]),
        output: template::Output::Build(vec![template::Destination {
            particle: template::Particle::Build(vec![template::Value::Reference(slot.clone())]),
            body: None,
        }]),
    };
    let mut value = construction.literal("Seed");
    for depth in 0..32 {
        let initial = Environment::new(scope.identity());
        let environment = Environment {
            value: initial.value.bind(&slot, value).unwrap(),
            ..initial
        };
        value = template.instantiate(&construction, &environment).unwrap();
        let mut cursor = value.value();
        for _ in 0..=depth {
            let structure::Value::Rule(rule) = cursor else {
                panic!("template instantiation must retain every constructed level");
            };
            cursor = &rule.output.destination()[0].particle.value()[0];
        }
        assert_eq!(cursor, &structure::Value::Atom("Seed".into()));
    }
}
