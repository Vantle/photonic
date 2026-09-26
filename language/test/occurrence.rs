use crate::basis::Set;
use crate::flow::Flow;
use crate::place::Place;
use crate::program::Symbol;
use crate::runtime::Limit;
use crate::state::{Frame, State, Token, World};
use std::sync::Arc;
use std::task::Poll;

fn token(id: usize, capture: usize) -> Token {
    Token {
        id,
        value: Symbol::Rule(0),
        capture: Some(capture),
    }
}

fn state() -> State {
    State {
        world: vec![Arc::new(World {
            frame: 1,
            particle: Vec::new(),
        })]
        .into(),
        frame: vec![
            Arc::new(Frame {
                scope: 0,
                parent: None,
                lexical: None,
                particle: vec![token(7, 0), token(11, 1)].into(),
                held: Vec::new(),
            }),
            Arc::new(Frame {
                scope: 1,
                parent: Some(0),
                lexical: Some(0),
                particle: vec![token(19, 1)].into(),
                held: vec![token(23, 0)],
            }),
        ]
        .into(),
    }
}

#[test]
fn identity() {
    let original = state();
    let mut renamed = original.clone();
    for index in 0..renamed.frame.len() {
        let frame = Arc::make_mut(&mut renamed.frame[index]);
        let mut particle = frame.particle.iter().cloned().collect::<Vec<_>>();
        particle.reverse();
        for token in &mut particle {
            token.id = 100 - token.id;
        }
        frame.particle = particle.into();
        for token in &mut frame.held {
            token.id = 100 - token.id;
        }
    }
    assert_eq!(original.canonical().state, renamed.canonical().state);
    assert_eq!(
        crate::fingerprint::state(&original),
        crate::fingerprint::state(&renamed)
    );
    assert_eq!(
        crate::fingerprint::signature(&original),
        crate::fingerprint::signature(&renamed)
    );
    let mut structure = crate::structure::Structure::default();
    assert_eq!(structure.advance(&original), structure.advance(&renamed));
    let mut changed = original.clone();
    let mut particle = changed.frame[0]
        .particle
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    particle[0].capture = Some(1);
    Arc::make_mut(&mut changed.frame[0]).particle = particle.into();
    assert_ne!(original.canonical().state, changed.canonical().state);
    Arc::make_mut(&mut changed.frame[0]).particle = vec![token(7, 0), token(11, 0)].into();
    let duplicate = changed.canonical().state;
    let particle = duplicate.frame[0].particle.iter().collect::<Vec<_>>();
    assert_eq!(particle.len(), 2);
    assert_ne!(particle[0].id, particle[1].id);
}

#[test]
fn visibility() {
    let state = state();
    assert_eq!(
        state.visible(0).map(|(place, _)| place).collect::<Set<_>>(),
        [Place::Context(0, 7), Place::Context(0, 11)]
            .into_iter()
            .collect()
    );
    assert_eq!(
        state.visible(1).map(|(place, _)| place).collect::<Set<_>>(),
        [
            Place::Context(0, 7),
            Place::Context(0, 11),
            Place::Context(1, 19)
        ]
        .into_iter()
        .collect()
    );
    assert_eq!(
        state.resolve(crate::location::Location::World(0), 7),
        Some(Place::Context(0, 7))
    );
    assert_eq!(
        state.resolve(crate::location::Location::World(0), 19),
        Some(Place::Context(1, 19))
    );
    assert_eq!(state.resolve(crate::location::Location::World(0), 23), None);
}

#[test]
fn projection() {
    let state = state();
    let flow = Flow::identity(&state);
    let binding = flow
        .project(
            &state,
            &state,
            &[crate::slot::Slot {
                location: crate::location::Location::World(0),
                token: vec![7, 19],
                position: 0,
            }],
            1,
            None,
        )
        .unwrap();
    let expected = [Place::Context(0, 7), Place::Context(1, 19)]
        .into_iter()
        .collect::<Set<_>>();
    assert_eq!(binding.world, Set::single(0));
    assert_eq!(binding.footprint, expected);
    assert_eq!(binding.exact, expected);
    let canonical = state.canonical();
    let expected = Flow::identity(&canonical.state);
    let result = flow.rename(canonical);
    assert_eq!(result.flow.resource.len(), expected.resource.len());
    for (place, basis) in result.flow.resource.iter() {
        assert_eq!(expected.resource[place], Set::single(*place));
        assert_eq!(basis.len(), 1);
        assert_eq!(
            result.state.token(*place).unwrap().value,
            state.token(*basis.first().unwrap()).unwrap().value
        );
    }
}

#[test]
fn retirement() {
    let original = state();
    let mut changed = original.clone();
    changed.world.clear();
    assert_eq!(changed.reachable(), vec![0, 1]);
    Arc::make_mut(&mut changed.frame[0])
        .particle
        .retain(|token| token.capture != Some(1));
    assert_eq!(changed.reachable(), vec![0]);
    let changed = changed.reclaim(&[0]);
    assert_eq!(changed.frame.len(), 1);
    assert_eq!(changed.size(), 1);
    assert_eq!(original.frame[0].particle.len(), 2);
    assert_eq!(original.size(), 4);
}

#[test]
fn allocation() {
    let original = state();
    let layout = crate::layout::Layout::new(&original);
    assert_eq!(layout.cell, 4);
    assert_eq!(layout.resource, 24);
    let mut changed = original.clone();
    Arc::make_mut(&mut changed.frame[1]).particle = original.frame[1]
        .particle
        .iter()
        .cloned()
        .chain([token(41, 0)])
        .collect();
    let change = crate::change::Change {
        world: Set::default(),
        insertion: 1..1,
        frame: vec![1],
    };
    let reach = layout.reach.advance(&original, &changed, &change);
    let advanced = layout.advance(&original, &changed, &change, reach);
    assert_eq!(advanced.resource, 42);
    assert_eq!(advanced.cell, changed.size());
    let fingerprint = crate::fingerprint::Index::new(Arc::new(original), &layout.reach.frame);
    let advanced = fingerprint.advance(Arc::new(changed.clone()), &change, &advanced.reach.frame);
    assert_eq!(advanced.value(), crate::fingerprint::state(&changed));
}

#[test]
fn consumption() {
    for context in [false, true] {
        let mut source = state();
        if !context {
            source.world.push(Arc::new(World {
                frame: 0,
                particle: vec![token(31, 0)],
            }));
            source.world.push(source.world[1].clone());
        }
        let selected = if context {
            Place::Context(0, 7)
        } else {
            Place::World(1, 31)
        };
        let binding = crate::flow::Binding {
            world: if context {
                Set::default()
            } else {
                Set::single(1)
            },
            footprint: Set::single(selected),
            exact: Set::single(selected),
            read: Set::single(selected),
        };
        let rule = crate::program::Instruction::default();
        let exhaustive = crate::application::apply(crate::application::Request {
            scope: &[],
            source: &source,
            frame: 0,
            owner: crate::application::Owner::Frame(0),
            rule: &rule,
            binding: &binding,
        });
        let layout = crate::layout::Layout::new(&source);
        let direct = crate::evaluation::apply(crate::evaluation::Request {
            source: &source,
            frame: 0,
            owner: 0,
            rule: &rule,
            scope: &[],
            state: source.clone(),
            next: layout.resource,
            binding: &binding,
            layout: &layout,
        });
        assert_eq!(
            direct.state.canonical().state,
            exhaustive.state.canonical().state
        );
        assert_eq!(direct.layout.cell, direct.state.size());
        assert_eq!(
            direct.layout.resource,
            crate::layout::Layout::new(&direct.state).resource
        );
        assert_eq!(
            direct.state.frame[0].particle.len(),
            if context { 1 } else { 2 }
        );
        assert_eq!(source.frame[0].particle.len(), 2);
        assert!(direct.state.world[0].particle.is_empty());
        if !context {
            assert_eq!(direct.state.world[1].particle, vec![token(31, 0)]);
        }
        assert!(
            exhaustive
                .flow
                .resource
                .iter()
                .all(|(place, _)| exhaustive.state.token(*place).is_some())
        );
        assert!(source.token(selected).is_some());
    }
}

#[test]
fn import() {
    let source = state();
    let mut endpoint = source.clone();
    endpoint.frame.push(Arc::new(Frame {
        scope: 2,
        parent: Some(0),
        lexical: Some(0),
        particle: vec![token(41, 2)].into(),
        held: Vec::new(),
    }));
    let identity = Flow::identity(&source);
    let flow = Flow {
        resource: identity
            .resource
            .into_iter()
            .chain([(Place::Context(2, 41), Set::single(Place::Context(0, 7)))])
            .collect(),
        context: identity.context,
        frame: vec![Some(0), Some(1), None],
    };
    let rule = crate::program::Instruction {
        output: vec![crate::program::Output::Particle(vec![Symbol::Rule(0)])],
        ..Default::default()
    };
    let binding = crate::flow::Binding {
        world: Set::default(),
        footprint: Set::default(),
        exact: Set::default(),
        read: Set::single(Place::Context(0, 7)),
    };
    let result = crate::application::apply(crate::application::Request {
        scope: &[],
        source: &source,
        frame: 0,
        owner: crate::application::Owner::Capture(crate::flow::Closure {
            state: &endpoint,
            flow: &flow,
            capture: 2,
        }),
        rule: &rule,
        binding: &binding,
    });
    assert_eq!(result.state.frame.len(), 3);
    assert_eq!(result.state.frame[2].particle.len(), 1);
    let imported = result.state.frame[2].particle.iter().next().unwrap();
    assert_eq!(imported.capture, Some(2));
    assert!(imported.id > 23);
    assert_eq!(result.state.world[1].particle[0].capture, Some(2));
    assert_ne!(result.state.world[1].particle[0].id, imported.id);
    assert_eq!(
        result.flow.resource[&Place::Context(2, imported.id)],
        Set::single(Place::Context(0, 7))
    );
    assert_eq!(result.state.reachable(), vec![0, 1, 2]);
    let canonical = result.canonical();
    assert_eq!(canonical.state.size(), 6);
    assert_eq!(canonical.flow.resource.len(), 6);
    assert_eq!(source.frame.len(), 2);
}

fn transition(search: &mut crate::reduction::Search) -> Option<crate::reduction::Event> {
    for _ in 0..100_000 {
        if let Poll::Ready(event) = search.run(crate::runtime::Limit {
            coherence: 64,
            ..crate::runtime::Limit::default()
        }) {
            return event;
        }
    }
    panic!("finite matching exceeded its work bound")
}

#[test]
fn loading() {
    let program = crate::program::Program::new(
        &frontend::lowering::parse("().([A] B), [A] B, [A] B").unwrap(),
    );
    let state = State::initial(&program);
    assert_eq!(state.world.len(), 1);
    assert_eq!(state.world[0].particle.len(), 1);
    assert_eq!(state.frame[0].particle.len(), 2);
    let occurrence = state.frame[0]
        .particle
        .iter()
        .chain(&state.world[0].particle)
        .collect::<Vec<_>>();
    assert!(
        occurrence
            .iter()
            .all(|token| token.value == occurrence[0].value)
    );
    assert!(occurrence.iter().all(|token| token.capture == Some(0)));
    assert_eq!(
        occurrence
            .iter()
            .map(|token| token.id)
            .collect::<Set<_>>()
            .len(),
        3
    );
    assert_eq!(program.rule.len(), 1);
}

#[test]
fn opening() {
    let program = crate::program::Program::new(
        &frontend::lowering::parse("Z, (X, (Y, [Y] W), [X] V)").unwrap(),
    );
    let state = State::initial(&program);
    assert_eq!(
        state
            .frame
            .iter()
            .map(|frame| (
                frame.scope,
                frame.parent,
                frame.lexical,
                frame.particle.len()
            ))
            .collect::<Vec<_>>(),
        [
            (0, None, None, 0),
            (1, Some(0), Some(0), 1),
            (2, Some(1), Some(1), 1)
        ]
    );
    assert!(state.frame.iter().all(|frame| frame.held.is_empty()));
    let atom = |name: &str| Symbol::Atom(program.atom.get_index_of(name).unwrap());
    assert_eq!(
        state
            .world
            .iter()
            .map(|world| (world.frame, world.particle[0].value))
            .collect::<Vec<_>>(),
        [(0, atom("Z")), (1, atom("X")), (2, atom("Y"))]
    );
    assert_eq!(state, state.canonical().state);
}

#[test]
fn startup() {
    for source in ["[] A", "().([] A)"] {
        let program = Arc::new(crate::program::Program::new(
            &frontend::lowering::parse(source).unwrap(),
        ));
        let mut state = Arc::new(State::initial(&program));
        let initial = state.world.len();
        let mut search = crate::reduction::Search::new(program.clone(), state.clone());
        for count in 1..=3 {
            let event = transition(&mut search).unwrap();
            assert_eq!(event.binding.world.len(), 0);
            assert_eq!(event.binding.footprint.len(), 0);
            assert_eq!(event.binding.read.len(), 1);
            assert_eq!(event.state.world.len(), initial + count);
            assert_eq!(event.state.world[initial + count - 1].particle.len(), 1);
            assert_eq!(event.state.frame[0].particle, state.frame[0].particle);
            state = event.state.clone();
            search.advance(event.state, &event.change, event.fingerprint, event.layout);
        }
        let mut runtime = crate::runtime::Runtime::new(&frontend::lowering::parse(source).unwrap());
        runtime.run(
            100_000,
            crate::runtime::Limit {
                configuration: 8,
                coherence: initial + 3,
                occurrence: 16,
                scope: 8,
                record: 100_000,
            },
        );
        assert!(
            runtime
                .state
                .iter()
                .any(|value| value.canonical().state == state.canonical().state)
        );
    }
}

#[test]
fn authority() {
    let program = Arc::new(crate::program::Program::new(
        &frontend::lowering::parse("[] A").unwrap(),
    ));
    let initial = State::initial(&program);
    let mut consumed = initial.clone();
    Arc::make_mut(&mut consumed.frame[0]).particle.clear();
    let consumed = Arc::new(consumed);
    let mut direct = crate::reduction::Search::new(program.clone(), consumed.clone());
    assert!(transition(&mut direct).is_none());
    let mut exhaustive = crate::runtime::Runtime::seed(program, consumed);
    exhaustive.run(100_000, Limit::default());
    assert!(exhaustive.closed());
    assert!(exhaustive.snapshot().event.is_empty());
    assert_eq!(initial.frame[0].particle.len(), 1);
}

#[test]
fn entry() {
    let program = Arc::new(crate::program::Program::new(
        &frontend::lowering::parse("Seed, [Seed] (B, [B] C)").unwrap(),
    ));
    let state = Arc::new(State::initial(&program));
    assert_eq!(state.frame.len(), 1);
    assert_eq!(state.frame[0].particle.len(), 1);
    let mut search = crate::reduction::Search::new(program, state);
    let event = transition(&mut search).unwrap();
    assert_eq!(event.state.frame.len(), 2);
    assert_eq!(event.state.frame[1].particle.len(), 1);
    assert_eq!(
        event.state.frame[1].particle.iter().next().unwrap().capture,
        Some(1)
    );
    search.advance(event.state, &event.change, event.fingerprint, event.layout);
    let event = transition(&mut search).unwrap();
    assert_eq!(event.state.frame.len(), 1);
    assert_eq!(event.state.world[0].frame, 0);
    assert_eq!(event.state.frame[0].particle.len(), 1);
}

#[test]
fn operand() {
    let source = "[A] B, [[A] B] C";
    let program = Arc::new(crate::program::Program::new(
        &frontend::lowering::parse(source).unwrap(),
    ));
    let initial = Arc::new(State::initial(&program));
    assert_eq!(initial.world.len(), 0);
    let mut direct = crate::reduction::Search::new(program.clone(), initial.clone());
    let event = transition(&mut direct).unwrap();
    assert_eq!(event.binding.world.len(), 0);
    assert_eq!(event.binding.footprint.len(), 1);
    assert!(matches!(
        event.binding.footprint.first(),
        Some(Place::Context(0, _))
    ));
    assert_eq!(event.state.frame[0].particle.len(), 1);
    assert_eq!(event.state.world.len(), 1);
    assert_eq!(
        event.state.world[0].particle[0].value,
        Symbol::Atom(program.atom.get_index_of("C").unwrap())
    );
    let expected = event.state.canonical().state;
    direct.advance(event.state, &event.change, event.fingerprint, event.layout);
    assert!(transition(&mut direct).is_none());
    let mut exhaustive = crate::runtime::Runtime::seed(program, initial.clone());
    exhaustive.run(100_000, Limit::default());
    assert!(exhaustive.closed());
    assert!(
        exhaustive
            .state
            .iter()
            .any(|state| state.canonical().state == expected)
    );
    assert_eq!(initial.frame[0].particle.len(), 2);
}

#[test]
fn multiplicity() {
    for source in [
        "[A] B, [([A] B).([A] B)] C",
        "(),(), [A] B, [[A] B,[A] B] C",
    ] {
        let program = Arc::new(crate::program::Program::new(
            &frontend::lowering::parse(source).unwrap(),
        ));
        let initial = Arc::new(State::initial(&program));
        let mut direct = crate::reduction::Search::new(program.clone(), initial.clone());
        assert!(transition(&mut direct).is_none(), "{source}");
        let mut exhaustive = crate::runtime::Runtime::seed(program, initial);
        exhaustive.run(100_000, Limit::default());
        assert!(exhaustive.closed());
        assert!(exhaustive.snapshot().event.is_empty(), "{source}");
    }
    let program = Arc::new(crate::program::Program::new(
        &frontend::lowering::parse("[A] B, [A] B, [([A] B).([A] B)] C").unwrap(),
    ));
    let initial = Arc::new(State::initial(&program));
    let mut direct = crate::reduction::Search::new(program, initial);
    let event = transition(&mut direct).unwrap();
    assert_eq!(event.binding.exact.len(), 2);
    assert_eq!(event.state.frame[0].particle.len(), 1);
}

#[test]
fn invalidation() {
    let program = Arc::new(crate::program::Program::new(
        &frontend::lowering::parse("[A] B, [A] B, [[A] B] C").unwrap(),
    ));
    let initial = Arc::new(State::initial(&program));
    let mut cached = crate::reduction::Search::new(program.clone(), initial);
    for remaining in [2, 1] {
        let event = transition(&mut cached).unwrap();
        assert_eq!(event.state.frame[0].particle.len(), remaining);
        let mut fresh = crate::reduction::Search::new(program.clone(), event.state.clone());
        let state = event.state.clone();
        cached.advance(event.state, &event.change, event.fingerprint, event.layout);
        cached.evict();
        assert_eq!(
            transition(&mut cached).map(|event| event.state.canonical().state),
            transition(&mut fresh).map(|event| event.state.canonical().state)
        );
        if remaining == 2 {
            cached = crate::reduction::Search::new(program.clone(), state);
        }
    }
    assert!(transition(&mut cached).is_none());
}

#[test]
fn remainder() {
    let mut source = state();
    source.world = vec![
        Arc::new(World {
            frame: 0,
            particle: vec![token(31, 0)],
        }),
        Arc::new(World {
            frame: 0,
            particle: vec![token(31, 0), token(37, 0)],
        }),
    ]
    .into();
    let selected = [Place::World(0, 31), Place::World(1, 37)]
        .into_iter()
        .collect::<Set<_>>();
    let binding = crate::flow::Binding {
        world: [0, 1].into_iter().collect(),
        footprint: selected.clone(),
        exact: selected,
        read: Set::single(Place::World(0, 31)),
    };
    let rule = crate::program::Instruction {
        output: vec![crate::program::Output::Particle(Vec::new())],
        ..Default::default()
    };
    let result = crate::application::apply(crate::application::Request {
        source: &source,
        scope: &[],
        frame: 0,
        owner: crate::application::Owner::Frame(0),
        rule: &rule,
        binding: &binding,
    });
    assert_eq!(result.state.world[0].particle, vec![token(31, 0)]);
    assert_eq!(
        result.flow.resource[&Place::World(0, 31)],
        Set::single(Place::World(1, 31))
    );
    assert_eq!(source.world[0].particle, vec![token(31, 0)]);
}

#[test]
fn target() {
    for (source, target, reached) in [
        ("A, [A] B", "B", false),
        ("A, [A] B", "B, [A] B", true),
        ("A, [A] B", "B, [A] B, [A] B", false),
        ("[A] B", "[A] B", true),
        ("[A] B", "[A] C", false),
        ("[A] B, [[A] B] C", "C, [[A] B] C", true),
        ("[A] B, [[A] B] C", "C, [A] B, [[A] B] C", false),
    ] {
        let mut direct = crate::path::Search::new(
            frontend::lowering::parse(source).unwrap(),
            Some(frontend::lowering::parse(target).unwrap()),
        );
        direct.run(100_000, crate::runtime::Limit::default());
        assert_eq!(
            direct.summary().outcome == crate::prism::Outcome::Reached,
            reached,
            "{source} => {target}"
        );
        let mut exhaustive =
            crate::runtime::Runtime::new(&frontend::lowering::parse(source).unwrap());
        exhaustive.run(100_000, Limit::default());
        assert_eq!(
            exhaustive
                .verdict(&frontend::lowering::parse(target).unwrap())
                .outcome
                == crate::prism::Outcome::Reached,
            reached,
            "{source} => {target}"
        );
        assert!(exhaustive.snapshot().closed);
    }
}

#[test]
fn mixed() {
    let source = format!("{},X, [B] C, [A.([B] C)] D, [X] A", vec!["A"; 40].join(","));
    let program = Arc::new(crate::program::Program::new(
        &frontend::lowering::parse(&source).unwrap(),
    ));
    let initial = Arc::new(State::initial(&program));
    let symbol = Symbol::Atom(program.atom.get_index_of("X").unwrap());
    let rule = program
        .rule
        .iter()
        .position(|rule| rule.input == vec![vec![symbol]])
        .unwrap();
    let mut cached = crate::reduction::Search::new(program.clone(), initial);
    let event = loop {
        let event = transition(&mut cached).unwrap();
        if event.rule == rule {
            break event;
        }
    };
    let mut fresh = crate::reduction::Search::new(program, event.state.clone());
    cached.advance(event.state, &event.change, event.fingerprint, event.layout);
    let drain = |search: &mut crate::reduction::Search| {
        std::iter::from_fn(|| transition(search))
            .map(|event| (event.rule, event.binding.world, event.binding.exact))
            .collect::<std::collections::HashSet<_>>()
    };
    let expected = drain(&mut fresh);
    assert_eq!(expected.len(), 41);
    assert_eq!(drain(&mut cached), expected);
}
