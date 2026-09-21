use model::admission::{self, Witness};
use model::application::{Code, Request, Selection};
use model::configuration::Configuration;
use model::construction::Construction;
use model::context::{self, Frame, Reference};
use model::flow::Place;
use model::fragment::Fragment;
use model::history::History;
use model::occurrence::{Identity, Occurrence};
use model::path::{Path, Step};
use model::structure::{Body, Destination, Output, Particle, Rule, Value};
use model::world::{self, World};
use std::collections::BTreeSet;

fn rule(context: u64, input: &str, output: &str) -> Rule {
    let Value::Rule(rule) = super::rule(context, input, Value::Atom(output.into())) else {
        unreachable!()
    };
    *rule
}

fn code(context: u64) -> Value {
    let mut declaration = model::activation::capture(rule(0, "Local", "Done"));
    declaration.context = Reference::Local(0);
    let mut rule = rule(context, "Call", "A");
    rule.output = Output::new(vec![Destination {
        particle: Particle::new(vec![Value::Atom("A".into())]),
        body: Some(Body::bind(context::Identity(context), vec![declaration]).unwrap()),
    }]);
    Value::Rule(Box::new(rule))
}

pub(super) fn initial(shared: bool) -> Path {
    let held = super::source(10, Value::Atom("Secret".into()));
    let mut occurrence = vec![
        super::source(0, code(1)),
        super::source(1, Value::Atom("Call".into())),
        super::source(2, Value::Atom("Keep".into())),
    ];
    if shared {
        occurrence.push(held.clone());
    }
    Path::new(
        Configuration::new(
            context::Identity(0),
            vec![World {
                identity: world::Identity(0),
                context: context::Identity(0),
                occurrence,
            }],
            vec![
                Frame {
                    identity: context::Identity(0),
                    parent: None,
                    lexical: None,
                    declaration: vec![rule(0, "A", "Global")],
                    held: vec![],
                },
                Frame {
                    identity: context::Identity(1),
                    parent: Some(context::Identity(0)),
                    lexical: Some(context::Identity(0)),
                    declaration: vec![rule(1, "A", "Local")],
                    held: vec![held],
                },
            ],
            History::default(),
        )
        .unwrap(),
    )
}

pub(super) fn retain(path: &Path, identity: Identity) -> Fragment<Value> {
    let occurrence = path
        .target()
        .world()
        .next()
        .unwrap()
        .occurrence
        .iter()
        .find(|value| value.identity == identity)
        .unwrap();
    Construction::new(context::Identity(0), History::default())
        .inspect(occurrence)
        .unwrap()
}

pub(super) fn remove(path: &Path, consumed: Vec<Identity>) -> Path {
    path.advance(Step::Introduction {
        world: path.target().world().next().unwrap().identity,
        consumed,
        value: Construction::new(context::Identity(0), History::default()).literal("Bridge"),
    })
    .unwrap()
}

pub(super) fn restore(
    path: &Path,
    value: Fragment<Value>,
    state: usize,
    occurrence: Identity,
) -> Path {
    let source = path.state()[state].world().next().unwrap().identity;
    let world = path.target().world().next().unwrap();
    let consumed = world
        .occurrence
        .iter()
        .filter(|value| value.value == Value::Atom("Bridge".into()))
        .map(|value| value.identity)
        .collect();
    path.advance(Step::Historical(admission::Request {
        world: world.identity,
        consumed,
        value,
        witness: vec![Witness {
            state,
            world: source,
            place: model::flow::Place::World(source, occurrence),
        }],
    }))
    .unwrap()
}

fn request(state: &Configuration, code: Code, input: &str) -> Request {
    let world = state.world().next().unwrap();
    let occurrence = world
        .occurrence
        .iter()
        .find(|value| value.value == Value::Atom(input.into()))
        .unwrap()
        .identity;
    Request {
        code,
        selection: vec![Selection {
            world: world.identity,
            occurrence: vec![occurrence],
        }],
    }
}

fn generated(state: &Configuration) -> &Occurrence {
    state
        .world()
        .next()
        .unwrap()
        .occurrence
        .iter()
        .find(|value| matches!(value.value, Value::Rule(_)))
        .unwrap()
}

fn validate(path: &Path) {
    for state in path.state() {
        Configuration::new(
            state.root(),
            state.world().cloned().collect(),
            state.frame().cloned().collect(),
            state.history().clone(),
        )
        .unwrap();
    }
}

#[test]
fn execution() {
    for shared in [false, true] {
        let initial = initial(shared);
        let value = retain(&initial, Identity(0));
        let erased = remove(&initial, vec![Identity(0)]);
        assert_eq!(erased.target().frame().count(), 1);
        let restored = restore(&erased, value, 0, Identity(0));
        let frame = restored
            .target()
            .frame()
            .find(|frame| frame.identity != context::Identity(0))
            .unwrap();
        assert_eq!(frame.identity, context::Identity(2));
        assert_eq!(frame.declaration, vec![rule(2, "A", "Local")]);
        assert_eq!(frame.held.len(), 1);
        assert_eq!(frame.held[0].identity == Identity(10), shared);
        assert_eq!(frame.held[0].value, Value::Atom("Secret".into()));
        let basis = &restored.record()[1].flow.resource
            [&Place::Held(frame.identity, frame.held[0].identity)];
        assert_eq!(
            basis,
            &if shared {
                BTreeSet::from([Place::World(world::Identity(1), Identity(10))])
            } else {
                BTreeSet::new()
            }
        );
        assert_eq!(
            restored.record()[1].archive[&context::Identity(2)].resource[&Identity(10)],
            frame.held[0].identity
        );
        assert_eq!(generated(restored.target()).value, code(2));
        let step = request(
            restored.target(),
            Code::Local {
                world: restored.target().world().next().unwrap().identity,
                occurrence: generated(restored.target()).identity,
            },
            "Call",
        );
        let entered = restored.advance(Step::Application(step)).unwrap();
        let site = entered.target().world().next().unwrap().context;
        assert_eq!(site, context::Identity(3));
        let frame = entered
            .target()
            .frame()
            .find(|frame| frame.identity == site)
            .unwrap();
        assert_eq!(frame.parent, Some(context::Identity(0)));
        assert_eq!(frame.lexical, Some(context::Identity(2)));
        let step = request(
            entered.target(),
            Code::Declaration {
                context: context::Identity(2),
                position: 0,
            },
            "A",
        );
        let local = entered.advance(Step::Application(step)).unwrap();
        let step = request(
            local.target(),
            Code::Declaration {
                context: site,
                position: 0,
            },
            "Local",
        );
        let result = local.advance(Step::Application(step)).unwrap();
        let world = result.target().world().next().unwrap();
        let done = world
            .occurrence
            .iter()
            .find(|value| value.value == Value::Atom("Done".into()))
            .unwrap();
        assert_eq!(world.context, context::Identity(0));
        assert_eq!(
            result.flow().resource[&Place::World(world.identity, done.identity)],
            BTreeSet::from([Place::World(world::Identity(0), Identity(1))])
        );
        assert_eq!(
            world
                .occurrence
                .iter()
                .filter(|value| value.value == Value::Atom("Secret".into()))
                .count(),
            usize::from(shared)
        );
        assert!(
            result
                .target()
                .world()
                .all(|world| world.context != context::Identity(2))
        );
        validate(&result);
    }
}

#[test]
fn sharing() {
    for cycle in [false, true] {
        let initial = initial(false);
        let mut world = initial.source().world().cloned().collect::<Vec<_>>();
        world[0].occurrence.push(super::source(20, code(2)));
        let mut frame = initial.source().frame().cloned().collect::<Vec<_>>();
        frame.push(Frame {
            identity: context::Identity(2),
            parent: Some(context::Identity(0)),
            lexical: Some(context::Identity(0)),
            declaration: vec![rule(2, "A", "Local")],
            held: frame[1].held.clone(),
        });
        if cycle {
            frame[1].lexical = Some(context::Identity(2));
            frame[2].lexical = Some(context::Identity(1));
            frame[1].held[0].value = super::rule(2, "Read", Value::Atom("Secret".into()));
            frame[2].held = frame[1].held.clone();
        }
        let initial = Path::new(
            Configuration::new(context::Identity(0), world, frame, History::default()).unwrap(),
        );
        let first = retain(&initial, Identity(0));
        let second = retain(&initial, Identity(20));
        let erased = remove(&initial, vec![Identity(0), Identity(20)]);
        assert_eq!(erased.target().frame().count(), 1);
        let restored = restore(&erased, first, 0, Identity(0));
        let result = restore(&restored, second, 0, Identity(20));
        let frame = result
            .target()
            .frame()
            .filter(|frame| frame.identity != context::Identity(0))
            .collect::<Vec<_>>();
        assert_eq!(frame.len(), 2);
        assert_eq!(frame[0].held, frame[1].held);
        assert_eq!(frame[0].held.len(), 1);
        assert_ne!(frame[0].held[0].identity, Identity(10));
        assert!(
            result
                .target()
                .world()
                .next()
                .unwrap()
                .occurrence
                .iter()
                .any(|value| value.value == code(3))
        );
        assert!(
            result
                .target()
                .world()
                .next()
                .unwrap()
                .occurrence
                .iter()
                .any(|value| value.value == code(4))
        );
        if cycle {
            assert_eq!(frame[0].lexical, Some(frame[1].identity));
            assert_eq!(frame[1].lexical, Some(frame[0].identity));
            assert!(result.record()[2].archive.is_empty());
            assert_eq!(
                frame[0].held[0].value,
                super::rule(4, "Read", Value::Atom("Secret".into()))
            );
        } else {
            assert_eq!(
                result.record()[2].flow.resource
                    [&Place::Held(frame[1].identity, frame[1].held[0].identity)],
                BTreeSet::from([Place::Held(frame[0].identity, frame[0].held[0].identity)])
            );
        }
        validate(&result);
    }
}

#[test]
fn inheritance() {
    let initial = initial(false);
    let original = retain(&initial, Identity(0));
    let erased = remove(&initial, vec![Identity(0)]);
    let restored = restore(&erased, original.clone(), 0, Identity(0));
    let identity = generated(restored.target()).identity;
    let copied = retain(&restored, identity);
    let erased = remove(&restored, vec![identity]);
    assert_eq!(erased.target().frame().count(), 1);
    let restored = restore(&erased, copied, 2, identity);
    assert_eq!(generated(restored.target()).value, code(3));
    let result = restore(&restored, original, 0, Identity(0));
    assert_eq!(result.target().frame().count(), 2);
    assert!(result.record().last().unwrap().archive.is_empty());
    assert_eq!(
        result
            .target()
            .world()
            .next()
            .unwrap()
            .occurrence
            .iter()
            .filter(|value| value.value == code(3))
            .count(),
        2
    );
    validate(&result);
}

#[test]
fn equivalence() {
    let initial = initial(false);
    let original = retain(&initial, Identity(0));
    let erased = remove(&initial, vec![Identity(0)]);
    let restored = restore(&erased, original.clone(), 0, Identity(0));
    let identity = generated(restored.target()).identity;
    let copied = retain(&restored, identity);
    let erased = remove(&restored, vec![identity]);
    let construction = Construction::new(context::Identity(0), History::default());
    let input = construction
        .input([construction
            .particle([construction.literal("Call")])
            .unwrap()])
        .unwrap();
    let output = construction
        .output([construction
            .destination(construction.particle([original, copied]).unwrap(), None)
            .unwrap()])
        .unwrap();
    let value = construction.rule(input, output).unwrap();
    let world = erased.target().world().next().unwrap();
    let result = erased
        .advance(Step::Historical(admission::Request {
            world: world.identity,
            consumed: vec![],
            value,
            witness: vec![
                Witness {
                    state: 0,
                    world: world::Identity(0),
                    place: model::flow::Place::World(world::Identity(0), Identity(0)),
                },
                Witness {
                    state: 2,
                    world: world::Identity(2),
                    place: model::flow::Place::World(world::Identity(2), identity),
                },
            ],
        }))
        .unwrap();
    assert_eq!(result.target().frame().count(), 2);
    let Value::Rule(rule) = &generated(result.target()).value else {
        panic!()
    };
    assert_eq!(
        rule.output.destination()[0].particle.value(),
        &[code(3), code(3)]
    );
    assert_eq!(result.record()[3].archive.len(), 1);
    assert_eq!(
        result.record()[3].archive[&context::Identity(3)].context,
        context::Identity(2)
    );
    validate(&result);
}

#[test]
fn capacity() {
    for context in [false, true] {
        let initial = initial(false);
        let mut world = initial.source().world().cloned().collect::<Vec<_>>();
        let mut frame = initial.source().frame().cloned().collect::<Vec<_>>();
        if context {
            frame.push(Frame {
                identity: context::Identity(u64::MAX),
                parent: Some(context::Identity(0)),
                lexical: Some(context::Identity(0)),
                declaration: vec![],
                held: vec![],
            });
        } else {
            world[0]
                .occurrence
                .push(super::source(u64::MAX - 1, Value::Atom("Limit".into())));
        }
        let initial = Path::new(
            Configuration::new(context::Identity(0), world, frame, History::default()).unwrap(),
        );
        let value = retain(&initial, Identity(0));
        let erased = remove(&initial, vec![Identity(0)]);
        let original = erased.clone();
        assert_eq!(
            erased.advance(Step::Historical(admission::Request {
                world: world::Identity(1),
                consumed: vec![],
                value,
                witness: vec![Witness {
                    state: 0,
                    world: world::Identity(0),
                    place: model::flow::Place::World(world::Identity(0), Identity(0))
                }],
            })),
            Err(model::failure::Failure::Capacity)
        );
        assert_eq!(erased, original);
        assert_eq!(erased.target().frame().count(), 1);
    }
}
