use model::application::{self, Code, Request, Selection};
use model::configuration::Configuration;
use model::context::{self, Frame};
use model::history::History;
use model::occurrence::{self, Occurrence};
use model::structure::{Body, Destination, Input, Output, Particle, Rule, Value};
use model::world::{self, World};
use photonic::lowering::parse;
use photonic::path::{Report, Search};
use photonic::prism::Outcome;
use photonic::runtime::Limit;
use std::collections::BTreeMap;

#[derive(Debug, Eq, PartialEq)]
struct Observation {
    world: Vec<Vec<String>>,
    occurrence: Vec<(String, Vec<usize>)>,
}

fn observe(mut world: Vec<Vec<(u64, String)>>) -> Observation {
    for particle in &mut world {
        particle.sort_by(|left, right| left.1.cmp(&right.1));
    }
    world.sort_by_key(|particle| {
        particle
            .iter()
            .map(|(_, value)| value.clone())
            .collect::<Vec<_>>()
    });
    let mut occurrence = BTreeMap::new();
    for (position, particle) in world.iter().enumerate() {
        for (identity, value) in particle {
            occurrence
                .entry(identity)
                .or_insert_with(|| (value.clone(), Vec::new()))
                .1
                .push(position);
        }
    }
    let mut occurrence = occurrence.into_values().collect::<Vec<_>>();
    occurrence.sort();
    Observation {
        world: world
            .into_iter()
            .map(|particle| particle.into_iter().map(|(_, value)| value).collect())
            .collect(),
        occurrence,
    }
}

fn native(source: &str, target: &str) -> Report {
    let mut search = Search::new(parse(source).unwrap(), parse(target).unwrap()).unwrap();
    search.run(100_000, Limit::default());
    search.report()
}

fn check(state: &Configuration, report: &Report) {
    compare(state, &report.state[report.event.last().unwrap().target]);
}

fn compare(state: &Configuration, witness: &photonic::snapshot::Node) {
    let model = observe(
        state
            .world()
            .map(|world| {
                world
                    .occurrence
                    .iter()
                    .map(|occurrence| {
                        (
                            occurrence.identity.0,
                            match &occurrence.value {
                                Value::Atom(value) => value.clone(),
                                Value::Rule(_) => "Rule".into(),
                            },
                        )
                    })
                    .collect()
            })
            .collect(),
    );
    let native = observe(
        witness
            .world
            .iter()
            .map(|world| {
                world
                    .particle
                    .iter()
                    .map(|token| {
                        (
                            token.id as u64,
                            if token.capture.is_some() {
                                "Rule".into()
                            } else {
                                token.label.clone()
                            },
                        )
                    })
                    .collect()
            })
            .collect(),
    );
    assert_eq!(model, native);
    assert_eq!(state.frame().count(), witness.frame.len());
}

fn atom(value: &str) -> Value {
    Value::Atom(value.into())
}

fn initial(value: Vec<Value>, declaration: Vec<Rule>) -> Configuration {
    Configuration::new(
        context::Identity(0),
        vec![World {
            identity: world::Identity(0),
            context: context::Identity(0),
            occurrence: value
                .into_iter()
                .enumerate()
                .map(|(identity, value)| Occurrence {
                    identity: occurrence::Identity(identity as u64),
                    value,
                    history: History::default(),
                })
                .collect(),
        }],
        vec![Frame {
            identity: context::Identity(0),
            parent: None,
            lexical: None,
            declaration,
            held: Vec::new(),
        }],
        History::default(),
    )
    .unwrap()
}

#[test]
fn branching() {
    for width in 1..=3 {
        for branch in 1..=3 {
            for retained in 0..=2 {
                let pattern = vec!["A"; width].join(".");
                let remainder = ".Keep".repeat(retained);
                let output = vec!["B"; branch].join(",");
                let source = format!("{pattern}{remainder} [{pattern}] {output}");
                let target = vec![format!("B{remainder}"); branch].join(",");
                let report = native(&source, &target);
                assert_eq!(
                    report.outcome,
                    if branch > 1 && retained > 0 {
                        Outcome::Unknown
                    } else {
                        Outcome::Reached
                    },
                    "{source}"
                );
                assert_eq!(report.event.len(), 1);
                let rule = Rule {
                    context: context::Identity(0),
                    input: Input::new(vec![Particle::new(vec![atom("A"); width])]),
                    output: Output::new(vec![
                        Destination {
                            particle: Particle::new(vec![atom("B")]),
                            body: None
                        };
                        branch
                    ]),
                };
                let state = initial(
                    vec![atom("A"); width]
                        .into_iter()
                        .chain(vec![atom("Keep"); retained])
                        .collect(),
                    vec![rule],
                );
                let event = application::apply(
                    &state,
                    &Request {
                        code: Code::Declaration {
                            context: context::Identity(0),
                            position: 0,
                        },
                        selection: vec![Selection {
                            world: world::Identity(0),
                            occurrence: (0..width as u64).map(occurrence::Identity).collect(),
                        }],
                    },
                )
                .unwrap();
                check(&event.target, &report);
                assert_eq!(event.consumed.len(), report.event[0].footprint.len());
                assert_eq!(report.event[0].exact, report.event[0].footprint);
                assert_eq!(event.read.len(), report.event[0].read.len());
            }
        }
    }
}

#[test]
fn executable() {
    let report = native("A.([A] B)", "B.([A] B)");
    assert_eq!(report.outcome, Outcome::Reached);
    let rule = Rule {
        context: context::Identity(0),
        input: Input::new(vec![Particle::new(vec![atom("A")])]),
        output: Output::new(vec![Destination {
            particle: Particle::new(vec![atom("B")]),
            body: None,
        }]),
    };
    let state = initial(vec![atom("A"), Value::Rule(Box::new(rule))], Vec::new());
    let event = application::apply(
        &state,
        &Request {
            code: Code::Local {
                world: world::Identity(0),
                occurrence: occurrence::Identity(1),
            },
            selection: vec![Selection {
                world: world::Identity(0),
                occurrence: vec![occurrence::Identity(0)],
            }],
        },
    )
    .unwrap();
    check(&event.target, &report);
    assert_eq!(report.event.len(), 1);
    assert_eq!(report.event[0].footprint.len(), 1);
    assert_eq!(report.event[0].read.len(), 1);
    assert!(
        report.event[0]
            .read
            .iter()
            .all(|place| !report.event[0].footprint.contains(place))
    );
    assert_eq!(event.read.len(), report.event[0].read.len());
}

#[test]
fn joining() {
    let report = native("Seed.Keep [Seed] A,B [A.Keep,B.Keep] C", "C");
    assert_eq!(report.outcome, Outcome::Reached);
    assert_eq!(report.event.len(), 2);
    assert_eq!(report.event[1].footprint.len(), 4);
    let split = Rule {
        context: context::Identity(0),
        input: Input::new(vec![Particle::new(vec![atom("Seed")])]),
        output: Output::new(
            ["A", "B"]
                .into_iter()
                .map(|value| Destination {
                    particle: Particle::new(vec![atom(value)]),
                    body: None,
                })
                .collect(),
        ),
    };
    let join = Rule {
        context: context::Identity(0),
        input: Input::new(
            ["A", "B"]
                .into_iter()
                .map(|value| Particle::new(vec![atom(value), atom("Keep")]))
                .collect(),
        ),
        output: Output::new(vec![Destination {
            particle: Particle::new(vec![atom("C")]),
            body: None,
        }]),
    };
    let state = initial(vec![atom("Seed"), atom("Keep")], vec![split, join]);
    let split = application::apply(
        &state,
        &Request {
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
    let joined = application::apply(
        &split.target,
        &Request {
            code: Code::Declaration {
                context: context::Identity(0),
                position: 1,
            },
            selection: vec![
                Selection {
                    world: world::Identity(1),
                    occurrence: vec![occurrence::Identity(2), occurrence::Identity(1)],
                },
                Selection {
                    world: world::Identity(2),
                    occurrence: vec![occurrence::Identity(3), occurrence::Identity(1)],
                },
            ],
        },
    )
    .unwrap();
    assert_eq!(joined.consumed.len(), report.event[1].footprint.len());
    check(&joined.target, &report);
}

#[test]
fn returning() {
    let report = native("A [A] (X [X] Y)", "Y");
    assert_eq!(report.outcome, Outcome::Reached);
    assert_eq!(report.event.len(), 2);
    let returner = Rule {
        context: context::Identity(0),
        input: Input::new(vec![Particle::new(vec![atom("X")])]),
        output: Output::new(vec![Destination {
            particle: Particle::new(vec![atom("Y")]),
            body: None,
        }]),
    };
    let entry = Rule {
        context: context::Identity(0),
        input: Input::new(vec![Particle::new(vec![atom("A")])]),
        output: Output::new(vec![Destination {
            particle: Particle::new(vec![atom("X")]),
            body: Some(Body::new(context::Identity(0), vec![returner])),
        }]),
    };
    let state = initial(vec![atom("A")], vec![entry]);
    let entered = application::apply(
        &state,
        &Request {
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
    let returned = application::apply(
        &entered.target,
        &Request {
            code: Code::Declaration {
                context: context::Identity(1),
                position: 0,
            },
            selection: vec![Selection {
                world: world::Identity(1),
                occurrence: vec![occurrence::Identity(1)],
            }],
        },
    )
    .unwrap();
    check(&returned.target, &report);
    let intermediate = &report.state[report.event[0].target];
    assert_eq!(intermediate.world[0].frame, 1);
    assert_eq!(intermediate.frame[1].parent, Some(0));
    assert_eq!(intermediate.frame[1].lexical, Some(0));
    assert_eq!(intermediate.frame[1].held.len(), 1);
    assert_eq!(intermediate.frame[1].held[0].label, "A");
    assert_eq!(returned.consumed.len(), report.event[1].footprint.len());
}

fn declared(state: &Configuration, context: u64, input: &str) -> Code {
    let frame = state
        .frame()
        .find(|frame| frame.identity.0 == context)
        .unwrap();
    let expected = Input::new(vec![Particle::new(vec![atom(input)])]);
    let position = frame
        .declaration
        .iter()
        .position(|rule| rule.input == expected)
        .unwrap();
    Code::Declaration {
        context: context::Identity(context),
        position,
    }
}

fn step(state: &Configuration, code: Code, input: &str) -> application::Event {
    assert_eq!(state.world().count(), 1);
    let world = state.world().next().unwrap();
    let occurrence = world
        .occurrence
        .iter()
        .find(|value| value.value == atom(input))
        .unwrap();
    application::apply(
        state,
        &Request {
            code,
            selection: vec![Selection {
                world: world.identity,
                occurrence: vec![occurrence.identity],
            }],
        },
    )
    .unwrap()
}

#[test]
fn escaping() {
    let source = "Enter.Make.Call [Enter] ([A] Local, [Make] [Call] (A [Local] Done),) [A] Global";
    let report = native(source, "Missing");
    let witness = report
        .state
        .iter()
        .find(|state| {
            state
                .world
                .iter()
                .any(|world| world.particle.iter().any(|token| token.label == "Done"))
        })
        .unwrap();
    let initial = program::read(source);
    let entered = step(&initial, declared(&initial, 0, "Enter"), "Enter");
    let made = step(
        &entered.target,
        declared(&entered.target, 1, "Make"),
        "Make",
    );
    let world = made.target.world().next().unwrap();
    assert_eq!(world.context, context::Identity(0));
    let code = world
        .occurrence
        .iter()
        .find(|value| matches!(&value.value, Value::Rule(_)))
        .unwrap();
    let Value::Rule(rule) = &code.value else {
        unreachable!()
    };
    assert_eq!(rule.context, context::Identity(1));
    let called = step(
        &made.target,
        Code::Local {
            world: world.identity,
            occurrence: code.identity,
        },
        "Call",
    );
    assert_eq!(called.read.len(), 1);
    let local = step(&called.target, declared(&called.target, 1, "A"), "A");
    let done = step(&local.target, declared(&local.target, 2, "Local"), "Local");
    compare(&done.target, witness);
    assert_eq!(done.target.frame().count(), 2);
    let capture = witness.world[0]
        .particle
        .iter()
        .find_map(|token| token.capture)
        .unwrap();
    assert_ne!(capture, 0);
    assert_eq!(witness.frame[capture].parent, Some(0));
    assert_eq!(witness.frame[capture].lexical, Some(0));
}
mod program;
