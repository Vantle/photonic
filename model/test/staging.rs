use model::argument::Argument;
use model::capture::Capture;
use model::construction::Construction;
use model::environment::Environment;
use model::failure::Failure;
use model::fragment::Fragment;
use model::generator::{Definition, Generator, Invocation, Product, Term};
use model::history::History;
use model::machine::Machine;
use model::parameter::{Declaration, Parameter};
use model::path::{Path, Step};
use model::qualification::Qualification;
use model::scope::Scope;
use model::structure::Value;
use model::{context, scope, template};
use std::task::Poll;

fn rule(particle: template::Particle<Capture>) -> template::Value<Capture> {
    template::Value::Rule(Box::new(template::Rule {
        input: template::Input::Build(vec![template::Particle::Build(vec![
            template::Value::Atom("Tick".into()),
        ])]),
        output: template::Output::Build(vec![template::Destination {
            particle,
            body: None,
        }]),
    }))
}

fn check(path: &Path) {
    super::publication::validate(path);
    let expanded = path
        .advance(Step::Application(super::publication::opened(path)))
        .unwrap();
    super::publication::execute(&expanded);
    let inferred = Path::new(path.source().clone())
        .advance(Step::Inference {
            request: super::publication::opened(path),
            path: Box::new(path.clone()),
        })
        .unwrap();
    super::publication::execute(&inferred);
}

fn reclaim(path: &Path) -> Path {
    let world = path.target().world().next().unwrap();
    let code = world
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap();
    let erased = path
        .advance(Step::Introduction {
            world: world.identity,
            consumed: vec![code.identity],
            value: Construction::new(context::Identity(0), History::default()).literal("Bridge"),
        })
        .unwrap();
    assert_eq!(erased.target().frame().count(), 1);
    erased
}

fn resume(
    invocation: &Invocation<'_, Capture>,
    construction: &Construction<Capture, Capture>,
) -> Fragment<Value<Capture, Capture>> {
    let Product::Value(expected) = invocation.evaluate(construction).unwrap() else {
        panic!()
    };
    let mut machine = invocation.machine(construction).unwrap();
    assert_eq!(machine.run(100_000), Poll::Ready(Ok(expected.clone())));
    let work = machine.work();
    for boundary in 0..=work {
        let mut machine = invocation.machine(construction).unwrap();
        let result = machine.run(boundary);
        if boundary < work {
            assert_eq!(result, Poll::Pending);
        } else {
            assert_eq!(result, Poll::Ready(Ok(expected.clone())));
        }
        assert_eq!(
            machine.run(work - boundary),
            Poll::Ready(Ok(expected.clone()))
        );
        assert_eq!(machine.work(), work);
    }
    expected
}

#[test]
fn execution() {
    let (initial, witness) = super::qualification::branch();
    let qualification = Qualification::new(
        initial.clone(),
        initial.target().world().next().unwrap().identity,
    )
    .unwrap();
    let construction = qualification.construction();
    let mut scope = Scope::new(scope::Identity(0));
    let slot = scope.declare().unwrap();
    let environment = Environment::new(scope.identity());
    let environment = Environment {
        particle: environment
            .particle
            .bind(
                &slot,
                construction
                    .particle(
                        witness
                            .into_iter()
                            .map(|request| qualification.inspect(request).unwrap()),
                    )
                    .unwrap(),
            )
            .unwrap(),
        ..environment
    };
    let template = rule(template::Particle::Reference(slot));
    let expected = template.instantiate(construction, &environment).unwrap();
    assert_eq!(expected.evidence().qualified().count(), 2);
    let mut machine = Machine::new(&template, construction, &environment);
    assert_eq!(machine.run(100_000), Poll::Ready(Ok(expected.clone())));
    let work = machine.work();
    let published = super::publication::publish(&initial, expected.clone());
    check(&published);
    for boundary in 0..=work {
        let mut machine = Machine::new(&template, construction, &environment);
        if boundary < work {
            assert_eq!(machine.run(boundary), Poll::Pending);
        } else {
            assert_eq!(machine.run(boundary), Poll::Ready(Ok(expected.clone())));
        }
        let Poll::Ready(Ok(value)) = machine.run(work - boundary) else {
            panic!()
        };
        assert_eq!(super::publication::publish(&initial, value), published);
    }
    let mut path = published;
    for _ in 0..3 {
        let erased = reclaim(&path);
        path = super::publication::publish(&erased, expected.clone());
        check(&path);
    }
}

#[test]
fn factory() {
    let (path, witness) = super::qualification::branch();
    let qualification =
        Qualification::new(path.clone(), path.target().world().next().unwrap().identity).unwrap();
    let construction = qualification.construction();
    let mut outer = Scope::new(scope::Identity(1));
    let mut inner = Scope::new(scope::Identity(2));
    let left = outer.declare().unwrap();
    let right = inner.declare().unwrap();
    let unused = inner.declare().unwrap();
    let factory = Generator::close(
        Definition {
            declaration: Declaration::new(outer.identity(), vec![Parameter::Value(left.clone())])
                .unwrap(),
            term: Term::Quote(Box::new(Definition {
                declaration: Declaration::new(
                    inner.identity(),
                    vec![Parameter::Value(right.clone()), Parameter::Value(unused)],
                )
                .unwrap(),
                term: Term::Value(rule(template::Particle::Build(vec![
                    template::Value::Reference(left),
                    template::Value::Reference(right),
                ]))),
            })),
        },
        Environment::new(scope::Identity(0)),
        construction,
    )
    .unwrap();
    assert_eq!(factory.evidence().qualified().count(), 0);
    let invocation = factory
        .bind(
            construction,
            vec![Argument::Value(
                qualification.inspect(witness[0].clone()).unwrap(),
            )],
        )
        .unwrap();
    let Product::Generator(child) = invocation.evaluate(construction).unwrap() else {
        panic!()
    };
    assert_eq!(child.evidence().qualified().count(), 1);
    let world = path.target().world().next().unwrap();
    let tick = world
        .occurrence
        .iter()
        .find(|value| value.value == Value::Atom("Tick".into()))
        .unwrap();
    let argument = qualification
        .inspect(model::support::Request {
            address: model::support::Address {
                derivation: vec![],
                state: path.record().len(),
            },
            world: world.identity,
            place: model::flow::Place::World(world.identity, tick.identity),
        })
        .unwrap();
    let invocation = child
        .bind(
            construction,
            vec![
                Argument::Value(qualification.inspect(witness[1].clone()).unwrap()),
                Argument::Value(argument),
            ],
        )
        .unwrap();
    let value = resume(&invocation, construction);
    assert_eq!(value.evidence().qualified().count(), 3);
    let published = super::publication::publish(&path, value.clone());
    assert_eq!(
        published.record().last().unwrap().read,
        std::collections::BTreeSet::from([model::flow::Place::World(
            world.identity,
            tick.identity
        )])
    );
    check(&published);
    let restored = super::publication::publish(&reclaim(&published), value);
    check(&restored);
}

#[test]
fn sort() {
    let (path, witness) = super::qualification::branch();
    let qualification =
        Qualification::new(path.clone(), path.target().world().next().unwrap().identity).unwrap();
    let construction = qualification.construction();
    let left = qualification.inspect(witness[0].clone()).unwrap();
    let right = qualification.inspect(witness[1].clone()).unwrap();
    let mut scope = Scope::new(scope::Identity(1));
    let value = scope.declare().unwrap();
    let particle = scope.declare().unwrap();
    let input = scope.declare().unwrap();
    let output = scope.declare().unwrap();
    let body = scope.declare().unwrap();
    let template = template::Value::Rule(Box::new(template::Rule {
        input: template::Input::Build(vec![template::Particle::Build(vec![
            template::Value::Atom("Tick".into()),
        ])]),
        output: template::Output::Build(vec![template::Destination {
            particle: template::Particle::Build(vec![template::Value::Reference(value.clone())]),
            body: Some(template::Body::Build(vec![
                template::Rule {
                    input: template::Input::Reference(input.clone()),
                    output: template::Output::Reference(output.clone()),
                },
                template::Rule {
                    input: template::Input::Build(vec![template::Particle::Reference(
                        particle.clone(),
                    )]),
                    output: template::Output::Build(vec![template::Destination {
                        particle: template::Particle::Build(vec![template::Value::Reference(
                            value.clone(),
                        )]),
                        body: Some(template::Body::Reference(body.clone())),
                    }]),
                },
            ])),
        }]),
    }));
    let generator = Generator::close(
        Definition {
            declaration: Declaration::new(
                scope.identity(),
                vec![
                    Parameter::Value(value),
                    Parameter::Particle(particle),
                    Parameter::Input(input),
                    Parameter::Output(output),
                    Parameter::Body(body),
                ],
            )
            .unwrap(),
            term: Term::Value(template),
        },
        Environment::new(scope::Identity(0)),
        construction,
    )
    .unwrap();
    let invocation = generator
        .bind(
            construction,
            vec![
                Argument::Value(left.clone()),
                Argument::Particle(construction.particle([right.clone()]).unwrap()),
                Argument::Input(
                    construction
                        .input([construction.particle([left.clone()]).unwrap()])
                        .unwrap(),
                ),
                Argument::Output(
                    construction
                        .output([construction
                            .destination(construction.particle([right]).unwrap(), None)
                            .unwrap()])
                        .unwrap(),
                ),
                Argument::Body(
                    construction
                        .body([construction.definition(left).unwrap()])
                        .unwrap(),
                ),
            ],
        )
        .unwrap();
    let value = resume(&invocation, construction);
    assert_eq!(value.evidence().qualified().count(), 2);
    let published = super::publication::publish(&path, value.clone());
    for current in [
        &published,
        &super::publication::publish(&reclaim(&published), value),
    ] {
        super::publication::validate(current);
        let world = current.target().world().next().unwrap();
        let code = world
            .occurrence
            .iter()
            .find_map(|value| match &value.value {
                Value::Rule(rule) => Some(rule),
                _ => None,
            })
            .unwrap();
        let destination = &code.output.destination()[0];
        let Value::Rule(left) = &destination.particle.value()[0] else {
            panic!()
        };
        let declaration = destination
            .body
            .as_ref()
            .unwrap()
            .activate(context::Identity(99))
            .unwrap();
        assert_eq!(declaration.len(), 2);
        for rule in declaration {
            assert_eq!(rule.context, context::Identity(99));
            let Value::Rule(input) = &rule.input.particle()[0].value()[0] else {
                panic!()
            };
            let destination = &rule.output.destination()[0];
            let Value::Rule(output) = &destination.particle.value()[0] else {
                panic!()
            };
            assert_ne!(input.context, output.context);
            assert_ne!(input.context, context::Identity(99));
            assert_ne!(output.context, context::Identity(99));
            if let Some(body) = &destination.body {
                assert_eq!(
                    body.activate(context::Identity(100)).unwrap(),
                    vec![*left.clone()]
                );
                assert_eq!(output, left);
            } else {
                assert_eq!(input, left);
            }
        }
    }
}

#[test]
fn rejection() {
    let (path, witness) = super::qualification::branch();
    let world = path.target().world().next().unwrap().identity;
    let qualification = Qualification::new(path.clone(), world).unwrap();
    let detached = Qualification::new(Path::new(path.target().clone()), world).unwrap();
    let construction = qualification.construction();
    let mut scope = Scope::new(scope::Identity(0));
    let slot = scope.declare().unwrap();
    let template = rule(template::Particle::Build(vec![template::Value::Reference(
        slot.clone(),
    )]));
    let initial = Environment::new(scope.identity());
    let environment = Environment {
        value: initial
            .value
            .bind(&slot, qualification.inspect(witness[0].clone()).unwrap())
            .unwrap(),
        ..initial
    };
    assert_eq!(
        template.instantiate(detached.construction(), &environment),
        Err(Failure::Source)
    );
    super::machine::compare(&template, detached.construction(), &environment);
    let definition = Definition {
        declaration: Declaration::new(scope::Identity(1), vec![]).unwrap(),
        term: Term::Quote(Box::new(Definition {
            declaration: Declaration::new(scope::Identity(2), vec![]).unwrap(),
            term: Term::Value(template.clone()),
        })),
    };
    assert_eq!(
        Generator::close(definition, environment.clone(), detached.construction()),
        Err(Failure::Source)
    );
    let generator = Generator::close(
        Definition {
            declaration: Declaration::new(scope::Identity(1), vec![]).unwrap(),
            term: Term::Value(template),
        },
        environment,
        construction,
    )
    .unwrap();
    assert!(matches!(
        generator.bind(detached.construction(), vec![]),
        Err(Failure::Source)
    ));
    let invocation = generator.bind(construction, vec![]).unwrap();
    assert_eq!(
        invocation.evaluate(detached.construction()),
        Err(Failure::Source)
    );
    assert!(matches!(
        invocation.machine(detached.construction()),
        Err(Failure::Source)
    ));
    let mut parameter = Scope::new(scope::Identity(1));
    let ignored = parameter.declare().unwrap();
    let constant = Generator::close(
        Definition {
            declaration: Declaration::new(parameter.identity(), vec![Parameter::Value(ignored)])
                .unwrap(),
            term: Term::Value(template::Value::Atom("Constant".into())),
        },
        Environment::new(scope::Identity(0)),
        construction,
    )
    .unwrap();
    assert!(matches!(
        constant.bind(
            construction,
            vec![Argument::Value(detached.construction().literal("Unused"))]
        ),
        Err(Failure::Source)
    ));
}

#[test]
fn history() {
    let (path, _) = super::qualification::branch();
    let world = path.target().world().next().unwrap().identity;
    let qualify = |branch| {
        Qualification::new(
            Path::new(
                model::configuration::Configuration::new(
                    path.target().root(),
                    path.target().world().cloned().collect(),
                    path.target().frame().cloned().collect(),
                    History::default()
                        .decide(model::history::Identity(1), model::history::Branch(branch))
                        .unwrap(),
                )
                .unwrap(),
            ),
            world,
        )
        .unwrap()
    };
    let left = qualify(0);
    let right = qualify(1);
    let construction = left.construction();
    let mut scope = Scope::new(scope::Identity(0));
    let slot = scope.declare().unwrap();
    let template = rule(template::Particle::Build(vec![template::Value::Reference(
        slot.clone(),
    )]));
    let initial = Environment::new(scope.identity());
    assert!(matches!(
        template.instantiate(construction, &initial),
        Err(Failure::Binding { .. })
    ));
    super::machine::compare(&template, construction, &initial);
    let environment = Environment {
        value: initial
            .value
            .bind(&slot, right.construction().literal("Competing"))
            .unwrap(),
        ..initial
    };
    assert!(matches!(
        template.instantiate(construction, &environment),
        Err(Failure::History { .. })
    ));
    super::machine::compare(&template, construction, &environment);
    let generator = Generator::close(
        Definition {
            declaration: Declaration::new(scope::Identity(1), vec![]).unwrap(),
            term: Term::Value(template::Value::Atom("Constant".into())),
        },
        Environment::new(scope.identity()),
        construction,
    )
    .unwrap();
    assert!(matches!(
        generator.bind(right.construction(), vec![]),
        Err(Failure::History { .. })
    ));
    let invocation = generator.bind(construction, vec![]).unwrap();
    assert!(matches!(
        invocation.evaluate(right.construction()),
        Err(Failure::History { .. })
    ));
    assert!(matches!(
        invocation.machine(right.construction()),
        Err(Failure::History { .. })
    ));
}

fn local(
    input: &str,
    output: Vec<template::Value<Capture>>,
    body: Option<template::Body<Capture>>,
) -> template::Rule<Capture> {
    template::Rule {
        input: template::Input::Build(vec![template::Particle::Build(vec![
            template::Value::Atom(input.into()),
        ])]),
        output: template::Output::Build(vec![template::Destination {
            particle: template::Particle::Build(output),
            body,
        }]),
    }
}

fn apply(path: &Path, code: model::application::Code, label: &str) -> Path {
    let world = path.target().world().next().unwrap();
    let occurrence = world
        .occurrence
        .iter()
        .find(|value| value.value == Value::Atom(label.into()))
        .unwrap();
    path.advance(Step::Application(model::application::Request {
        code,
        selection: vec![model::application::Selection {
            world: world.identity,
            occurrence: vec![occurrence.identity],
        }],
    }))
    .unwrap()
}

#[test]
fn declaration() {
    let (path, witness) = super::qualification::branch();
    let qualification =
        Qualification::new(path.clone(), path.target().world().next().unwrap().identity).unwrap();
    let construction = qualification.construction();
    let mut scope = Scope::new(scope::Identity(0));
    let left = scope.declare().unwrap();
    let right = scope.declare().unwrap();
    let initial = Environment::new(scope.identity());
    let environment = Environment {
        value: initial
            .value
            .bind(&left, qualification.inspect(witness[0].clone()).unwrap())
            .unwrap()
            .bind(&right, qualification.inspect(witness[1].clone()).unwrap())
            .unwrap(),
        ..initial
    };
    let template = template::Value::Rule(Box::new(local(
        "Tick",
        vec![template::Value::Atom("Make".into())],
        Some(template::Body::Build(vec![
            local("Secret", vec![template::Value::Atom("Local".into())], None),
            local(
                "Make",
                vec![
                    template::Value::Reference(left),
                    template::Value::Reference(right),
                    template::Value::Rule(Box::new(local(
                        "Call",
                        vec![template::Value::Atom("Secret".into())],
                        Some(template::Body::Build(vec![])),
                    ))),
                ],
                None,
            ),
        ])),
    )));
    super::machine::compare(&template, construction, &environment);
    let value = template.instantiate(construction, &environment).unwrap();
    let published = super::publication::publish(&path, value.clone());
    let restored = super::publication::publish(&reclaim(&published), value);
    let mut entry = Vec::new();
    for path in [published, restored] {
        entry.push(
            path.advance(Step::Application(super::publication::opened(&path)))
                .unwrap(),
        );
        entry.push(
            Path::new(path.source().clone())
                .advance(Step::Inference {
                    request: super::publication::opened(&path),
                    path: Box::new(path),
                })
                .unwrap(),
        );
    }
    for entered in entry {
        let world = entered.target().world().next().unwrap();
        let frame = entered
            .target()
            .frame()
            .find(|frame| frame.identity == world.context)
            .unwrap();
        let position = frame
            .declaration
            .iter()
            .position(|rule| rule.input.particle()[0].value() == [Value::Atom("Make".into())])
            .unwrap();
        let expanded = apply(
            &entered,
            model::application::Code::Declaration {
                context: frame.identity,
                position,
            },
            "Make",
        );
        let world = expanded.target().world().next().unwrap();
        let mut observed = std::collections::BTreeSet::new();
        for value in &world.occurrence {
            let Value::Rule(rule) = &value.value else {
                continue;
            };
            if rule.context == context::Identity(0) {
                continue;
            }
            let called = apply(
                &expanded,
                model::application::Code::Local {
                    world: world.identity,
                    occurrence: value.identity,
                },
                "Call",
            );
            let frame = called
                .target()
                .frame()
                .find(|frame| frame.identity == rule.context)
                .unwrap();
            let position = frame
                .declaration
                .iter()
                .position(|rule| rule.input.particle()[0].value() == [Value::Atom("Secret".into())])
                .unwrap();
            let result = apply(
                &called,
                model::application::Code::Declaration {
                    context: frame.identity,
                    position,
                },
                "Secret",
            );
            let world = result.target().world().next().unwrap();
            let output = world
                .occurrence
                .iter()
                .max_by_key(|value| value.identity)
                .unwrap();
            let Value::Atom(label) = &output.value else {
                panic!()
            };
            observed.insert(label.clone());
            assert_eq!(
                result.flow().resource[&model::flow::Place::World(world.identity, output.identity)],
                std::collections::BTreeSet::from([model::flow::Place::World(
                    model::world::Identity(0),
                    model::occurrence::Identity(1)
                )])
            );
            super::publication::validate(&result);
        }
        assert_eq!(
            observed,
            std::collections::BTreeSet::from(["Left".into(), "Local".into(), "Right".into()])
        );
    }
}
