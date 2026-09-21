use model::argument::Argument;
use model::construction::Construction;
use model::environment::Environment;
use model::failure::Failure;
use model::generator::{Definition, Generator, Product, Term};
use model::history::{Branch, History};
use model::occurrence::{Identity, Occurrence};
use model::parameter::{Declaration, Parameter, Sort};
use model::scope::Scope;
use model::{context, scope, structure, template};
use std::collections::BTreeSet;
use std::task::Poll;

fn environment() -> Environment {
    Environment::new(scope::Identity(100))
}

#[test]
fn factory() {
    let construction = Construction::new(context::Identity(0), History::default());
    let mut outer = Scope::new(scope::Identity(1));
    let mut inner = Scope::new(scope::Identity(2));
    let role = outer.declare().unwrap();
    let payload = inner.declare().unwrap();
    let definition = Definition {
        declaration: Declaration::new(outer.identity(), vec![Parameter::Value(role.clone())])
            .unwrap(),
        term: Term::Quote(Box::new(Definition {
            declaration: Declaration::new(
                inner.identity(),
                vec![Parameter::Value(payload.clone())],
            )
            .unwrap(),
            term: Term::Value(template::Value::Rule(Box::new(template::Rule {
                input: template::Input::Build(vec![template::Particle::Build(vec![
                    template::Value::Reference(role),
                ])]),
                output: template::Output::Build(vec![template::Destination {
                    particle: template::Particle::Build(vec![template::Value::Reference(payload)]),
                    body: None,
                }]),
            }))),
        })),
    };
    let factory = Generator::close(definition, environment(), &construction).unwrap();
    assert!(factory.evidence().read().is_empty());
    let mut generated = Vec::new();
    for (identity, role) in [(1, "Left"), (2, "Right")] {
        let occurrence = super::source(identity, structure::Value::Atom(role.into()));
        let invocation = factory
            .bind(
                &construction,
                vec![Argument::Value(construction.inspect(&occurrence).unwrap())],
            )
            .unwrap();
        let Product::Generator(generator) = invocation.evaluate(&construction).unwrap() else {
            panic!("factory must construct a generator with a future binding");
        };
        assert_eq!(
            generator.evidence().read(),
            &BTreeSet::from([Identity(identity)])
        );
        generated.push((generator, role, identity));
    }
    let payload = super::source(
        3,
        super::rule(7, "Call", structure::Value::Atom("X".into())),
    );
    for (generator, role, identity) in generated {
        let invocation = generator
            .bind(
                &construction,
                vec![Argument::Value(construction.inspect(&payload).unwrap())],
            )
            .unwrap();
        let expected = invocation.evaluate(&construction).unwrap();
        let Product::Value(value) = expected else {
            panic!("generated builder must produce ordinary closed code");
        };
        assert_eq!(value.value(), &super::rule(0, role, payload.value.clone()));
        assert_eq!(
            value.evidence().read(),
            &BTreeSet::from([Identity(identity), Identity(3)])
        );
        let mut machine = invocation.machine(&construction).unwrap();
        while machine.run(1).is_pending() {}
        assert_eq!(machine.run(0), Poll::Ready(Ok(value)));
    }
}

#[test]
fn dangling() {
    let construction = Construction::new(context::Identity(0), History::default());
    let mut scope = Scope::new(scope::Identity(1));
    let declared = scope.declare().unwrap();
    let missing = scope.declare().unwrap();
    let definition = Definition {
        declaration: Declaration::new(scope.identity(), vec![Parameter::Value(declared)]).unwrap(),
        term: Term::Value(template::Value::Reference(missing)),
    };
    assert_eq!(
        Generator::close(definition, environment(), &construction),
        Err(Failure::Binding {
            scope: scope.identity(),
            position: 1,
        })
    );
    let mut foreign = Scope::new(scope::Identity(99));
    let definition = Definition {
        declaration: Declaration::new(scope.identity(), vec![]).unwrap(),
        term: Term::Value(template::Value::Reference(foreign.declare().unwrap())),
    };
    assert_eq!(
        Generator::close(definition, environment(), &construction),
        Err(Failure::Scope(foreign.identity()))
    );
}

#[test]
fn hygiene() {
    let construction = Construction::new(context::Identity(0), History::default());
    for identity in [0, 1, 7, 91] {
        let mut scope = Scope::new(scope::Identity(identity));
        let slot = scope.declare().unwrap();
        let definition = Definition {
            declaration: Declaration::new(scope.identity(), vec![Parameter::Value(slot.clone())])
                .unwrap(),
            term: Term::Value(template::Value::Reference(slot)),
        };
        let generator = Generator::close(definition, environment(), &construction).unwrap();
        let invocation = generator
            .bind(
                &construction,
                vec![Argument::Value(construction.literal("A"))],
            )
            .unwrap();
        assert_eq!(
            invocation.evaluate(&construction).unwrap(),
            Product::Value(construction.literal("A"))
        );
    }
    let scope = scope::Identity(1);
    let definition = Definition {
        declaration: Declaration::new(scope, vec![]).unwrap(),
        term: Term::Quote(Box::new(Definition {
            declaration: Declaration::new(scope, vec![]).unwrap(),
            term: Term::Value(template::Value::Atom("A".into())),
        })),
    };
    assert_eq!(
        Generator::close(definition, environment(), &construction),
        Err(Failure::Scope(scope))
    );
}

#[test]
fn signature() {
    let construction = Construction::new(context::Identity(0), History::default());
    let mut scope = Scope::new(scope::Identity(1));
    let slot = scope.declare().unwrap();
    assert_eq!(
        Declaration::new(
            scope.identity(),
            vec![
                Parameter::Value(slot.clone()),
                Parameter::Value(slot.clone())
            ],
        ),
        Err(Failure::Occupied {
            scope: scope.identity(),
            position: 0,
        })
    );
    assert_eq!(
        Declaration::new(scope::Identity(9), vec![Parameter::Value(slot.clone())]),
        Err(Failure::Scope(scope.identity()))
    );
    let declaration =
        Declaration::new(scope.identity(), vec![Parameter::Value(slot.clone())]).unwrap();
    let generator = Generator::close(
        Definition {
            declaration: declaration.clone(),
            term: Term::Value(template::Value::Reference(slot)),
        },
        environment(),
        &construction,
    )
    .unwrap();
    assert!(matches!(
        generator.bind(&construction, vec![]),
        Err(Failure::Arity {
            expected: 1,
            actual: 0
        })
    ));
    assert!(matches!(
        generator.bind(
            &construction,
            vec![Argument::Input(construction.input([]).unwrap())]
        ),
        Err(Failure::Sort {
            expected: Sort::Value,
            actual: Sort::Input
        })
    ));
    let mut conflicting = Scope::new(scope.identity());
    let particle = conflicting.declare().unwrap();
    let definition = Definition {
        declaration,
        term: Term::Value(template::Value::Rule(Box::new(template::Rule {
            input: template::Input::Build(vec![template::Particle::Reference(particle)]),
            output: template::Output::Build(vec![]),
        }))),
    };
    assert_eq!(
        Generator::close(definition, environment(), &construction),
        Err(Failure::Sort {
            expected: Sort::Value,
            actual: Sort::Particle
        })
    );
}

#[test]
fn capture() {
    let construction = Construction::new(context::Identity(0), History::default());
    let mut root = Scope::new(scope::Identity(100));
    let slot = root.declare().unwrap();
    let occurrence = super::source(
        3,
        super::rule(7, "Call", structure::Value::Atom("X".into())),
    );
    let initial = environment();
    let environment = Environment {
        value: initial
            .value
            .bind(&slot, construction.inspect(&occurrence).unwrap())
            .unwrap(),
        ..initial
    };
    let generator = Generator::close(
        Definition {
            declaration: Declaration::new(scope::Identity(1), vec![]).unwrap(),
            term: Term::Quote(Box::new(Definition {
                declaration: Declaration::new(scope::Identity(2), vec![]).unwrap(),
                term: Term::Value(template::Value::Reference(slot)),
            })),
        },
        environment,
        &construction,
    )
    .unwrap();
    assert_eq!(generator.evidence().read(), &BTreeSet::from([Identity(3)]));
    assert!(
        generator
            .evidence()
            .context()
            .contains(&context::Identity(7))
    );
    let invocation = generator.bind(&construction, vec![]).unwrap();
    let Product::Generator(child) = invocation.evaluate(&construction).unwrap() else {
        panic!("quotation must retain its captured reference");
    };
    let Product::Value(value) = child
        .bind(&construction, vec![])
        .unwrap()
        .evaluate(&construction)
        .unwrap()
    else {
        panic!("final stage must emit a closed value");
    };
    assert_eq!(value.value(), &occurrence.value);
    assert_eq!(value.evidence().read(), &BTreeSet::from([Identity(3)]));
}

#[test]
fn history() {
    let branch = History::default()
        .decide(model::history::Identity(1), Branch(0))
        .unwrap();
    let construction = Construction::new(context::Identity(0), branch.clone());
    let root = Construction::new(context::Identity(0), History::default());
    let mut scope = Scope::new(scope::Identity(1));
    let slot = scope.declare().unwrap();
    let definition = Definition {
        declaration: Declaration::new(scope.identity(), vec![Parameter::Value(slot)]).unwrap(),
        term: Term::Value(template::Value::Atom("Constant".into())),
    };
    let generator = Generator::close(definition, environment(), &root).unwrap();
    let source = Occurrence {
        identity: Identity(3),
        value: structure::Value::Atom("Argument".into()),
        history: branch,
    };
    let value = construction.inspect(&source).unwrap();
    assert!(
        generator
            .bind(&root, vec![Argument::Value(value.clone())])
            .is_err()
    );
    let invocation = generator
        .bind(&construction, vec![Argument::Value(value)])
        .unwrap();
    assert!(invocation.evaluate(&root).is_err());
    assert!(invocation.machine(&root).is_err());
    let Product::Value(value) = invocation.evaluate(&construction).unwrap() else {
        panic!("constant generator must emit a value");
    };
    assert_eq!(value.evidence().read(), &BTreeSet::from([Identity(3)]));
    let mut machine = invocation.machine(&construction).unwrap();
    assert_eq!(machine.run(1), Poll::Ready(Ok(value)));
}

#[test]
fn quotation() {
    let construction = Construction::new(context::Identity(0), History::default());
    let branch = History::default()
        .decide(model::history::Identity(1), Branch(0))
        .unwrap();
    let future = Construction::new(context::Identity(0), branch);
    let mut root = Scope::new(scope::Identity(100));
    let slot = root.declare().unwrap();
    let initial = environment();
    let captured = Environment {
        value: initial
            .value
            .bind(&slot, future.literal("Unavailable"))
            .unwrap(),
        ..initial
    };
    let definition = Definition {
        declaration: Declaration::new(scope::Identity(1), vec![]).unwrap(),
        term: Term::Quote(Box::new(Definition {
            declaration: Declaration::new(scope::Identity(2), vec![]).unwrap(),
            term: Term::Value(template::Value::Reference(slot)),
        })),
    };
    assert!(matches!(
        Generator::close(definition.clone(), captured.clone(), &construction),
        Err(Failure::History { actual: None, .. })
    ));
    let generator = Generator::close(definition, captured, &future).unwrap();
    assert!(generator.bind(&construction, vec![]).is_err());
    let invocation = generator.bind(&future, vec![]).unwrap();
    assert!(matches!(
        invocation.machine(&future),
        Err(Failure::Generator)
    ));
    let Product::Generator(child) = invocation.evaluate(&future).unwrap() else {
        panic!("quotation must retain its stage history");
    };
    assert!(child.bind(&construction, vec![]).is_err());
    assert!(child.bind(&future, vec![]).is_ok());
}

#[test]
fn sort() {
    let construction = Construction::new(context::Identity(0), History::default());
    let mut scope = Scope::new(scope::Identity(1));
    let particle = scope.declare().unwrap();
    let input = scope.declare().unwrap();
    let output = scope.declare().unwrap();
    let body = scope.declare().unwrap();
    let definition = Definition {
        declaration: Declaration::new(
            scope.identity(),
            vec![
                Parameter::Particle(particle.clone()),
                Parameter::Input(input.clone()),
                Parameter::Output(output.clone()),
                Parameter::Body(body.clone()),
            ],
        )
        .unwrap(),
        term: Term::Value(template::Value::Rule(Box::new(template::Rule {
            input: template::Input::Reference(input),
            output: template::Output::Build(vec![template::Destination {
                particle: template::Particle::Build(vec![template::Value::Rule(Box::new(
                    template::Rule {
                        input: template::Input::Build(vec![template::Particle::Reference(
                            particle,
                        )]),
                        output: template::Output::Reference(output),
                    },
                ))]),
                body: Some(template::Body::Reference(body)),
            }]),
        }))),
    };
    let generator = Generator::close(definition, environment(), &construction).unwrap();
    let invocation = generator
        .bind(
            &construction,
            vec![
                Argument::Particle(construction.particle([construction.literal("A")]).unwrap()),
                Argument::Input(construction.input([]).unwrap()),
                Argument::Output(construction.output([]).unwrap()),
                Argument::Body(construction.body([]).unwrap()),
            ],
        )
        .unwrap();
    let Product::Value(expected) = invocation.evaluate(&construction).unwrap() else {
        panic!("typed argument substitution must return ordinary code");
    };
    let structure::Value::Rule(rule) = expected.value() else {
        panic!("root must be a rule");
    };
    assert!(rule.input.particle().is_empty());
    assert!(rule.output.destination()[0].body.is_some());
    let mut machine = invocation.machine(&construction).unwrap();
    let result = machine.run(1000);
    assert_eq!(result, Poll::Ready(Ok(expected.clone())));
    let work = machine.work();
    for boundary in 0..work {
        let mut machine = invocation.machine(&construction).unwrap();
        assert_eq!(machine.run(boundary), Poll::Pending);
        assert_eq!(
            machine.run(work - boundary),
            Poll::Ready(Ok(expected.clone()))
        );
    }
}
