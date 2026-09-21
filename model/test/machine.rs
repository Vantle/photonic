use model::construction::Construction;
use model::environment::Environment;
use model::history::{Branch, History};
use model::machine::Machine;
use model::scope::Scope;
use model::{context, scope, template};
use std::task::Poll;

fn compare(value: &template::Value, construction: &Construction, environment: &Environment) {
    let expected = value.instantiate(construction, environment);
    let mut complete = Machine::new(value, construction, environment);
    assert_eq!(complete.run(0), Poll::Pending);
    assert_eq!(complete.work(), 0);
    assert_eq!(complete.run(100_000), Poll::Ready(expected.clone()));
    let work = complete.work();
    assert!(work > 0);
    assert_eq!(complete.run(0), Poll::Ready(expected.clone()));
    assert_eq!(complete.run(100_000), Poll::Ready(expected.clone()));
    assert_eq!(complete.work(), work);
    for boundary in 0..=work {
        let mut paused = Machine::new(value, construction, environment);
        let result = paused.run(boundary);
        assert_eq!(paused.work(), boundary);
        if boundary < work {
            assert_eq!(result, Poll::Pending);
        } else {
            assert_eq!(result, Poll::Ready(expected.clone()));
        }
        assert_eq!(paused.run(work - boundary), Poll::Ready(expected.clone()));
        assert_eq!(paused.work(), work);
    }
    for budget in [1, 2, 3, 7, 32] {
        let mut resumed = Machine::new(value, construction, environment);
        loop {
            let previous = resumed.work();
            let result = resumed.run(budget);
            assert!(resumed.work() - previous <= budget);
            assert!(resumed.work() <= work);
            if result.is_ready() {
                assert_eq!(result, Poll::Ready(expected.clone()));
                assert_eq!(resumed.work(), work);
                break;
            }
        }
    }
}

fn literal(value: &str) -> template::Value {
    template::Value::Atom(value.into())
}

fn wrapper(value: template::Value) -> template::Value {
    template::Value::Rule(Box::new(template::Rule {
        input: template::Input::Build(vec![template::Particle::Build(vec![literal("Step")])]),
        output: template::Output::Build(vec![template::Destination {
            particle: template::Particle::Build(vec![value]),
            body: None,
        }]),
    }))
}

#[test]
fn empty() {
    let construction = Construction::new(context::Identity(0), History::default());
    let environment = Environment::new(scope::Identity(0));
    compare(&literal("Atom"), &construction, &environment);
    for input in [vec![], vec![template::Particle::Build(vec![])]] {
        for output in [
            vec![],
            vec![template::Destination {
                particle: template::Particle::Build(vec![]),
                body: None,
            }],
            vec![template::Destination {
                particle: template::Particle::Build(vec![]),
                body: Some(template::Body::Build(vec![])),
            }],
        ] {
            compare(
                &template::Value::Rule(Box::new(template::Rule {
                    input: template::Input::Build(input.clone()),
                    output: template::Output::Build(output),
                })),
                &construction,
                &environment,
            );
        }
    }
}

#[test]
fn depth() {
    let construction = Construction::new(context::Identity(0), History::default());
    let environment = Environment::new(scope::Identity(0));
    let mut value = literal("Leaf");
    for _ in 0..8 {
        value = wrapper(value);
        compare(&value, &construction, &environment);
    }
}

#[test]
fn branching() {
    let construction = Construction::new(context::Identity(0), History::default());
    let environment = Environment::new(scope::Identity(0));
    let value = template::Value::Rule(Box::new(template::Rule {
        input: template::Input::Build(vec![
            template::Particle::Build(vec![literal("A"), wrapper(literal("B"))]),
            template::Particle::Build(vec![literal("C"), literal("C")]),
        ]),
        output: template::Output::Build(vec![
            template::Destination {
                particle: template::Particle::Build(vec![literal("X"), wrapper(literal("Y"))]),
                body: Some(template::Body::Build(vec![
                    template::Rule {
                        input: template::Input::Build(vec![]),
                        output: template::Output::Build(vec![]),
                    },
                    template::Rule {
                        input: template::Input::Build(vec![template::Particle::Build(vec![
                            literal("Z"),
                        ])]),
                        output: template::Output::Build(vec![]),
                    },
                ])),
            },
            template::Destination {
                particle: template::Particle::Build(vec![literal("Other")]),
                body: None,
            },
        ]),
    }));
    compare(&value, &construction, &environment);
}

#[test]
fn reference() {
    let construction = Construction::new(context::Identity(0), History::default());
    let mut scope = Scope::new(scope::Identity(0));
    let value = scope.declare().unwrap();
    let particle = scope.declare().unwrap();
    let input = scope.declare().unwrap();
    let output = scope.declare().unwrap();
    let body = scope.declare().unwrap();
    let initial = Environment::new(scope.identity());
    let environment = Environment {
        value: initial
            .value
            .bind(&value, construction.literal("X"))
            .unwrap(),
        particle: initial
            .particle
            .bind(&particle, construction.particle([]).unwrap())
            .unwrap(),
        input: initial
            .input
            .bind(&input, construction.input([]).unwrap())
            .unwrap(),
        output: initial
            .output
            .bind(&output, construction.output([]).unwrap())
            .unwrap(),
        body: initial
            .body
            .bind(&body, construction.body([]).unwrap())
            .unwrap(),
    };
    for template in [
        template::Value::Reference(value),
        template::Value::Rule(Box::new(template::Rule {
            input: template::Input::Reference(input),
            output: template::Output::Reference(output),
        })),
        template::Value::Rule(Box::new(template::Rule {
            input: template::Input::Build(vec![]),
            output: template::Output::Build(vec![template::Destination {
                particle: template::Particle::Reference(particle),
                body: Some(template::Body::Reference(body)),
            }]),
        })),
    ] {
        compare(&template, &construction, &environment);
    }
}

#[test]
fn failure() {
    let fork = model::history::Identity(1);
    let history = History::default().decide(fork, Branch(0)).unwrap();
    let origin = Construction::new(context::Identity(0), history);
    let construction = Construction::new(context::Identity(0), History::default());
    let mut scope = Scope::new(scope::Identity(0));
    let slot = scope.declare().unwrap();
    let initial = Environment::new(scope.identity());
    let value = wrapper(template::Value::Reference(slot.clone()));
    compare(&value, &construction, &initial);
    let environment = Environment {
        value: initial.value.bind(&slot, origin.literal("X")).unwrap(),
        ..initial
    };
    compare(&value, &construction, &environment);
    compare(&value, &origin, &environment);
    let foreign = Environment::new(scope::Identity(7));
    compare(&value, &construction, &foreign);
}
