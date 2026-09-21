use model::application::{self, Code, Place, Request, Selection};
use model::configuration::Configuration;
use model::context::{self, Frame};
use model::failure::Failure;
use model::history::{Branch, History};
use model::occurrence::{self, Occurrence};
use model::structure::{Body, Destination, Input, Output, Particle, Rule, Value};
use model::world::{self, World};
use std::collections::BTreeSet;

fn atom(value: &str) -> Value {
    Value::Atom(value.into())
}

fn rule(context: u64, input: &[&[&str]], output: &[&str]) -> Rule {
    Rule {
        context: context::Identity(context),
        input: Input::new(
            input
                .iter()
                .map(|particle| Particle::new(particle.iter().map(|value| atom(value)).collect()))
                .collect(),
        ),
        output: Output::new(
            output
                .iter()
                .map(|value| Destination {
                    particle: Particle::new(vec![atom(value)]),
                    body: None,
                })
                .collect(),
        ),
    }
}

fn frame(identity: u64, parent: Option<u64>, lexical: Option<u64>) -> Frame {
    Frame {
        identity: context::Identity(identity),
        parent: parent.map(context::Identity),
        lexical: lexical.map(context::Identity),
        declaration: Vec::new(),
        held: Vec::new(),
    }
}

fn source(identity: u64, value: Value) -> Occurrence {
    Occurrence {
        identity: occurrence::Identity(identity),
        value,
        history: History::default(),
    }
}

fn world(identity: u64, context: u64, occurrence: Vec<Occurrence>) -> World {
    World {
        identity: world::Identity(identity),
        context: context::Identity(context),
        occurrence,
    }
}

fn state(world: Vec<World>, frame: Vec<Frame>) -> Configuration {
    Configuration::new(context::Identity(0), world, frame, History::default()).unwrap()
}

fn selection(world: u64, occurrence: &[u64]) -> Selection {
    Selection {
        world: world::Identity(world),
        occurrence: occurrence
            .iter()
            .copied()
            .map(occurrence::Identity)
            .collect(),
    }
}

fn local(world: u64, occurrence: u64) -> Code {
    Code::Local {
        world: world::Identity(world),
        occurrence: occurrence::Identity(occurrence),
    }
}

fn declaration(context: u64) -> Code {
    Code::Declaration {
        context: context::Identity(context),
        position: 0,
    }
}

fn place(world: u64, occurrence: u64) -> Place {
    Place::World(world::Identity(world), occurrence::Identity(occurrence))
}

#[test]
fn direct() {
    let code = source(10, Value::Rule(Box::new(rule(0, &[&["A"]], &["B"]))));
    let original = world(9, 0, vec![source(0, atom("A"))]);
    let state = state(
        vec![
            world(
                0,
                0,
                vec![source(0, atom("A")), source(1, atom("Keep")), code.clone()],
            ),
            original.clone(),
        ],
        vec![frame(0, None, None)],
    );
    let event = application::apply(
        &state,
        &Request {
            code: local(0, 10),
            selection: vec![selection(0, &[0])],
        },
    )
    .unwrap();
    assert_eq!(event.read, BTreeSet::from([place(0, 10)]));
    assert_eq!(event.consumed, BTreeSet::from([place(0, 0)]));
    assert_eq!(event.owner, context::Identity(0));
    assert_eq!(
        event.target.world().find(|world| world.identity.0 == 9),
        Some(&original)
    );
    let result = event
        .target
        .world()
        .find(|world| world.identity.0 == 10)
        .unwrap();
    assert_eq!(
        result.occurrence,
        vec![source(1, atom("Keep")), code, source(11, atom("B"))]
    );
    assert_eq!(
        event.flow.resource[&place(10, 11)],
        BTreeSet::from([place(0, 0)])
    );
    assert_eq!(
        event.flow.resource[&place(10, 10)],
        BTreeSet::from([place(0, 10)])
    );
    assert_eq!(
        event.flow.context[&world::Identity(10)],
        BTreeSet::from([world::Identity(0)])
    );
    assert_eq!(state.world().count(), 2);
}

#[test]
fn shared() {
    let mut root = frame(0, None, None);
    root.declaration
        .push(rule(0, &[&["A"], &["A"]], &["B", "B"]));
    let occurrence = vec![source(0, atom("A")), source(1, atom("Keep"))];
    let state = state(
        vec![world(0, 0, occurrence.clone()), world(1, 0, occurrence)],
        vec![root],
    );
    let request = Request {
        code: declaration(0),
        selection: vec![selection(0, &[0]), selection(1, &[0])],
    };
    let event = application::apply(&state, &request).unwrap();
    let consumed = BTreeSet::from([place(0, 0), place(1, 0)]);
    assert_eq!(event.consumed, consumed);
    assert!(event.read.is_empty());
    let result = event.target.world().collect::<Vec<_>>();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].occurrence[0], source(1, atom("Keep")));
    assert_eq!(result[1].occurrence[0], result[0].occurrence[0]);
    assert_ne!(
        result[0].occurrence[1].identity,
        result[1].occurrence[1].identity
    );
    for world in result {
        assert_eq!(world.occurrence.len(), 2);
        assert_eq!(
            event.flow.resource[&Place::World(world.identity, occurrence::Identity(1))],
            BTreeSet::from([place(0, 1), place(1, 1)])
        );
        assert_eq!(
            event.flow.resource[&Place::World(world.identity, world.occurrence[1].identity)],
            consumed
        );
    }
    let duplicate = Request {
        selection: vec![selection(0, &[0]), selection(0, &[0])],
        ..request
    };
    assert_eq!(
        application::apply(&state, &duplicate),
        Err(Failure::World(world::Identity(0)))
    );
}

#[test]
fn multiplicity() {
    let mut root = frame(0, None, None);
    root.declaration.push(rule(0, &[&["A", "A"]], &["B"]));
    let state = state(
        vec![world(
            0,
            0,
            vec![
                source(0, atom("A")),
                source(1, atom("A")),
                source(2, atom("C")),
            ],
        )],
        vec![root],
    );
    for occurrence in [&[0, 1][..], &[1, 0][..]] {
        assert!(
            application::apply(
                &state,
                &Request {
                    code: declaration(0),
                    selection: vec![selection(0, occurrence)]
                }
            )
            .is_ok()
        );
    }
    for (occurrence, failure) in [
        (vec![0, 0], Failure::Repeated(occurrence::Identity(0))),
        (vec![0, 2], Failure::Match(world::Identity(0))),
        (vec![0, 9], Failure::Occurrence(occurrence::Identity(9))),
        (
            vec![0],
            Failure::Arity {
                expected: 2,
                actual: 1,
            },
        ),
    ] {
        assert_eq!(
            application::apply(
                &state,
                &Request {
                    code: declaration(0),
                    selection: vec![selection(0, &occurrence)]
                }
            ),
            Err(failure)
        );
    }
}

#[test]
fn locality() {
    let code = source(10, Value::Rule(Box::new(rule(0, &[&["A"]], &["B"]))));
    let state = state(
        vec![
            world(0, 0, vec![source(0, atom("A"))]),
            world(1, 0, vec![code]),
        ],
        vec![frame(0, None, None)],
    );
    assert_eq!(
        application::apply(
            &state,
            &Request {
                code: local(1, 10),
                selection: vec![selection(0, &[0])]
            }
        ),
        Err(Failure::World(world::Identity(1)))
    );
    assert_eq!(
        application::apply(
            &state,
            &Request {
                code: local(0, 0),
                selection: vec![selection(0, &[0])]
            }
        ),
        Err(Failure::Rule)
    );
    assert_eq!(
        application::apply(
            &state,
            &Request {
                code: local(0, 10),
                selection: Vec::new()
            }
        ),
        Err(Failure::Site)
    );
}

#[test]
fn empty() {
    for input in [vec![], vec![&[][..]], vec![&[][..], &[][..]]] {
        let mut root = frame(0, None, None);
        root.declaration.push(rule(0, &input, &["B"]));
        let state = state(
            vec![world(0, 0, Vec::new()), world(1, 0, Vec::new())],
            vec![root],
        );
        let request = Request {
            code: declaration(0),
            selection: (0..input.len().max(1) as u64)
                .map(|identity| selection(identity, &[]))
                .collect(),
        };
        let event = application::apply(&state, &request).unwrap();
        assert!(event.consumed.is_empty());
        assert_eq!(event.flow.resource[&place(2, 0)], BTreeSet::new());
        assert_eq!(
            event.flow.context[&world::Identity(2)].len(),
            input.len().max(1)
        );
    }
}

#[test]
fn scoped() {
    let mut entry = rule(0, &[&["A"]], &[]);
    entry.output = Output::new(vec![Destination {
        particle: Particle::new(vec![atom("X")]),
        body: Some(Body::new(
            context::Identity(0),
            vec![rule(0, &[&["X"]], &["Y"])],
        )),
    }]);
    let state = state(
        vec![world(
            0,
            0,
            vec![
                source(0, atom("A")),
                source(10, Value::Rule(Box::new(entry))),
            ],
        )],
        vec![frame(0, None, None)],
    );
    let entry = application::apply(
        &state,
        &Request {
            code: local(0, 10),
            selection: vec![selection(0, &[0])],
        },
    )
    .unwrap();
    let child = entry
        .target
        .frame()
        .find(|frame| frame.identity.0 == 1)
        .unwrap();
    assert_eq!(child.parent, Some(context::Identity(0)));
    assert_eq!(child.lexical, Some(context::Identity(0)));
    assert_eq!(child.held, vec![source(0, atom("A"))]);
    assert_eq!(
        entry.flow.resource[&Place::Held(context::Identity(1), occurrence::Identity(0))],
        BTreeSet::from([place(0, 0)])
    );
    let result = application::apply(
        &entry.target,
        &Request {
            code: declaration(1),
            selection: vec![selection(1, &[11])],
        },
    )
    .unwrap();
    assert_eq!(result.owner, context::Identity(1));
    assert_eq!(
        result.target.world().next().unwrap().context,
        context::Identity(0)
    );
    assert_eq!(result.target.frame().count(), 1);
    assert_eq!(result.flow.frame.len(), 1);
    assert_eq!(
        result.flow.resource[&place(2, 12)],
        BTreeSet::from([
            place(1, 11),
            Place::Held(context::Identity(1), occurrence::Identity(0))
        ])
    );
    assert!(
        result
            .flow
            .resource
            .keys()
            .all(|place| !matches!(place, Place::Held(_, _)))
    );
}

#[test]
fn lexical() {
    for owner in [0, 1, 2] {
        let mut root = frame(0, None, None);
        let mut lexical = frame(1, None, Some(0));
        let mut child = frame(2, Some(0), Some(1));
        root.declaration.push(rule(0, &[&["A"]], &["Root"]));
        lexical.declaration.push(rule(1, &[&["A"]], &["Lexical"]));
        child.declaration.push(rule(1, &[&["A"]], &["Child"]));
        let state = state(
            vec![world(0, 2, vec![source(0, atom("A"))])],
            vec![root, lexical, child],
        );
        let event = application::apply(
            &state,
            &Request {
                code: declaration(owner),
                selection: vec![selection(0, &[0])],
            },
        )
        .unwrap();
        assert_eq!(event.owner, context::Identity(owner));
        assert_eq!(
            event.target.world().next().unwrap().context,
            context::Identity(if owner == 2 { 0 } else { 2 })
        );
    }
    let mut root = frame(0, None, None);
    root.declaration.push(rule(0, &[&["A"]], &["Root"]));
    let state = state(
        vec![world(0, 2, vec![source(0, atom("A"))])],
        vec![root, frame(1, None, Some(2)), frame(2, Some(0), Some(1))],
    );
    assert_eq!(
        application::apply(
            &state,
            &Request {
                code: declaration(0),
                selection: vec![selection(0, &[0])]
            }
        ),
        Err(Failure::Context(context::Identity(0)))
    );
}

#[test]
fn captured() {
    for owner in [0, 1] {
        let code = source(10, Value::Rule(Box::new(rule(owner, &[&["A"]], &["B"]))));
        let state = state(
            vec![world(0, 1, vec![source(0, atom("A")), code])],
            vec![frame(0, None, None), frame(1, Some(0), Some(0))],
        );
        let event = application::apply(
            &state,
            &Request {
                code: local(0, 10),
                selection: vec![selection(0, &[0])],
            },
        )
        .unwrap();
        assert_eq!(
            event.target.world().next().unwrap().context,
            context::Identity(if owner == 1 { 0 } else { 1 })
        );
        assert_eq!(event.target.frame().count(), 2);
    }
}

#[test]
fn lifetime() {
    let executable = Value::Rule(Box::new(rule(1, &[&["A"]], &["B"])));
    let mut root = frame(0, None, None);
    root.declaration.push(Rule {
        context: context::Identity(0),
        input: Input::new(vec![Particle::new(vec![executable.clone()])]),
        output: Output::new(vec![Destination {
            particle: Particle::new(vec![atom("Gone")]),
            body: None,
        }]),
    });
    let state = state(
        vec![world(
            0,
            0,
            vec![source(0, atom("A")), source(10, executable)],
        )],
        vec![root, frame(1, None, None)],
    );
    let first = application::apply(
        &state,
        &Request {
            code: local(0, 10),
            selection: vec![selection(0, &[0])],
        },
    )
    .unwrap();
    let second = application::apply(
        &first.target,
        &Request {
            code: declaration(0),
            selection: vec![selection(1, &[10])],
        },
    )
    .unwrap();
    assert_eq!(first.read, BTreeSet::from([place(0, 10)]));
    assert_eq!(
        second.target.world().next().unwrap().occurrence,
        vec![source(11, atom("B")), source(12, atom("Gone"))]
    );
    assert_eq!(
        second.flow.resource[&place(2, 11)],
        BTreeSet::from([place(1, 11)])
    );
}

#[test]
fn validation() {
    let root = frame(0, None, None);
    let repeated = vec![world(
        0,
        0,
        vec![source(0, atom("A")), source(0, atom("A"))],
    )];
    assert_eq!(
        Configuration::new(
            context::Identity(0),
            repeated,
            vec![root.clone()],
            History::default()
        ),
        Err(Failure::Repeated(occurrence::Identity(0)))
    );
    let conflict = vec![
        world(0, 0, vec![source(0, atom("A"))]),
        world(1, 0, vec![source(0, atom("B"))]),
    ];
    assert_eq!(
        Configuration::new(
            context::Identity(0),
            conflict,
            vec![root.clone()],
            History::default()
        ),
        Err(Failure::Identity(occurrence::Identity(0)))
    );
    let dangling = vec![world(
        0,
        0,
        vec![source(0, Value::Rule(Box::new(rule(7, &[], &[]))))],
    )];
    assert_eq!(
        Configuration::new(
            context::Identity(0),
            dangling,
            vec![root.clone()],
            History::default()
        ),
        Err(Failure::Context(context::Identity(7)))
    );
    let fork = model::history::Identity(0);
    let past = History::default().decide(fork, Branch(0)).unwrap();
    let future = History::default().decide(fork, Branch(1)).unwrap();
    let mut value = source(0, atom("A"));
    value.history = past;
    assert_eq!(
        Configuration::new(
            context::Identity(0),
            vec![world(0, 0, vec![value])],
            vec![root],
            future
        ),
        Err(Failure::History {
            identity: fork,
            expected: Branch(0),
            actual: Some(Branch(1))
        })
    );
}

#[test]
fn capacity() {
    let mut root = frame(0, None, None);
    root.declaration = vec![rule(0, &[&["A"]], &[]), rule(0, &[&["A"]], &["B"])];
    let state = state(
        vec![world(u64::MAX, 0, vec![source(u64::MAX, atom("A"))])],
        vec![root],
    );
    let mut request = Request {
        code: declaration(0),
        selection: vec![selection(u64::MAX, &[u64::MAX])],
    };
    assert_eq!(
        application::apply(&state, &request)
            .unwrap()
            .target
            .world()
            .count(),
        0
    );
    request.code = Code::Declaration {
        context: context::Identity(0),
        position: 1,
    };
    assert_eq!(application::apply(&state, &request), Err(Failure::Capacity));
}

#[test]
fn construction() {
    let construction =
        model::construction::Construction::new(context::Identity(0), History::default());
    let mut frame = vec![
        frame(0, None, None),
        frame(1, None, Some(0)),
        frame(2, None, Some(0)),
    ];
    let mut payload = Vec::new();
    for (owner, label) in [(1, "Left"), (2, "Right")] {
        frame[owner as usize]
            .declaration
            .push(rule(owner, &[&["A"]], &[label]));
        let mut closure = rule(owner, &[&["Fire"]], &[]);
        closure.output = Output::new(vec![Destination {
            particle: Particle::new(vec![atom("A")]),
            body: Some(Body::new(context::Identity(owner), Vec::new())),
        }]);
        payload.push(
            construction
                .inspect(&source(owner, Value::Rule(Box::new(closure))))
                .unwrap(),
        );
    }
    payload.push(construction.literal("Fire"));
    let generated = construction
        .rule(
            construction
                .input([construction
                    .particle([construction.literal("Seed")])
                    .unwrap()])
                .unwrap(),
            construction
                .output([construction
                    .destination(construction.particle(payload).unwrap(), None)
                    .unwrap()])
                .unwrap(),
        )
        .unwrap();
    assert_eq!(
        generated.evidence().read(),
        &BTreeSet::from([occurrence::Identity(1), occurrence::Identity(2)])
    );
    let state = state(
        vec![world(
            0,
            0,
            vec![
                source(0, atom("Seed")),
                source(10, generated.value().clone()),
            ],
        )],
        frame,
    );
    let built = application::apply(
        &state,
        &Request {
            code: local(0, 10),
            selection: vec![selection(0, &[0])],
        },
    )
    .unwrap();
    for (owner, label) in [(1, "Left"), (2, "Right")] {
        let world = built.target.world().next().unwrap();
        let closure = world
            .occurrence
            .iter()
            .find(|value| matches!(&value.value, Value::Rule(rule) if rule.context.0 == owner))
            .unwrap();
        let fire = world
            .occurrence
            .iter()
            .find(|value| value.value == atom("Fire"))
            .unwrap();
        let entered = application::apply(
            &built.target,
            &Request {
                code: local(world.identity.0, closure.identity.0),
                selection: vec![selection(world.identity.0, &[fire.identity.0])],
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
            Some(context::Identity(owner))
        );
        let argument = world
            .occurrence
            .iter()
            .find(|value| value.value == atom("A"))
            .unwrap();
        let result = application::apply(
            &entered.target,
            &Request {
                code: declaration(owner),
                selection: vec![selection(world.identity.0, &[argument.identity.0])],
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
                .any(|value| value.value == atom(label))
        );
    }
}

#[test]
fn reclamation() {
    let mut root = frame(0, None, None);
    let enter = Rule {
        context: context::Identity(0),
        input: Input::new(vec![Particle::new(vec![atom("A")])]),
        output: Output::new(vec![Destination {
            particle: Particle::new(vec![atom("X")]),
            body: Some(Body::new(
                context::Identity(0),
                vec![rule(0, &[&["X"]], &["A"])],
            )),
        }]),
    };
    root.declaration.push(enter);
    let mut state = state(vec![world(0, 0, vec![source(0, atom("A"))])], vec![root]);
    let mut previous = 0;
    for _ in 0..8 {
        let world = state.world().next().unwrap();
        let entered = application::apply(
            &state,
            &Request {
                code: declaration(0),
                selection: vec![selection(
                    world.identity.0,
                    &[world.occurrence[0].identity.0],
                )],
            },
        )
        .unwrap();
        let world = entered.target.world().next().unwrap();
        assert!(world.context.0 > previous);
        previous = world.context.0;
        let returned = application::apply(
            &entered.target,
            &Request {
                code: declaration(world.context.0),
                selection: vec![selection(
                    world.identity.0,
                    &[world.occurrence[0].identity.0],
                )],
            },
        )
        .unwrap();
        assert_eq!(returned.target.frame().count(), 1);
        state = returned.target;
    }
}
