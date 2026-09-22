use crate::basis::Set;
use crate::flow::{Flow, Place};
use crate::program::Symbol;
use crate::state::{Frame, State, Token, World};
use std::sync::Arc;

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
                particle: vec![token(7, 0), token(11, 1)],
                held: Vec::new(),
            }),
            Arc::new(Frame {
                scope: 1,
                parent: Some(0),
                lexical: Some(0),
                particle: vec![token(19, 1)],
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
        frame.particle.reverse();
        for token in frame.particle.iter_mut().chain(&mut frame.held) {
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
    Arc::make_mut(&mut changed.frame[0]).particle[0].capture = Some(1);
    assert_ne!(original.canonical().state, changed.canonical().state);
    Arc::make_mut(&mut changed.frame[0]).particle = vec![token(7, 0), token(11, 0)];
    let duplicate = changed.canonical().state;
    assert_eq!(duplicate.frame[0].particle.len(), 2);
    assert_ne!(
        duplicate.frame[0].particle[0].id,
        duplicate.frame[0].particle[1].id
    );
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
    assert_eq!(state.resolve(0, 7), Some(Place::Context(0, 7)));
    assert_eq!(state.resolve(0, 19), Some(Place::Context(1, 19)));
    assert_eq!(state.resolve(0, 23), None);
}

#[test]
fn projection() {
    let state = state();
    let flow = Flow::identity(&state);
    let binding = flow
        .project(&state, &state, &[(0, vec![7, 19])], 1)
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
    Arc::make_mut(&mut changed.frame[1])
        .particle
        .push(token(41, 0));
    let change = crate::change::Change {
        world: Set::default(),
        insertion: 1..1,
        frame: vec![1],
    };
    let reach = layout.reach.advance(&original, &changed, &change);
    let advanced = layout.advance(&original, &changed, &change, reach);
    assert_eq!(advanced.resource, 42);
    assert_eq!(advanced.cell, changed.size());
    let fingerprint = crate::fingerprint::Index::new(Arc::new(original));
    let advanced = fingerprint.advance(Arc::new(changed.clone()), &change, advanced);
    assert_eq!(advanced.value, crate::fingerprint::state(&changed));
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
            source: &source,
            frame: 0,
            owner: Some(0),
            rule: &rule,
            binding: &binding,
            closure: None,
        });
        let layout = crate::layout::Layout::new(&source);
        let direct = crate::rewrite::apply(crate::rewrite::Request {
            source: &source,
            frame: 0,
            owner: 0,
            recipe: &crate::recipe::Recipe::new(&rule),
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
        particle: vec![token(41, 2)],
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
        output: vec![crate::program::Output {
            particle: vec![Symbol::Rule(0)],
            body: None,
        }],
        ..Default::default()
    };
    let binding = crate::flow::Binding {
        world: Set::default(),
        footprint: Set::default(),
        exact: Set::default(),
        read: Set::single(Place::Context(0, 7)),
    };
    let result = crate::application::apply(crate::application::Request {
        source: &source,
        frame: 0,
        owner: None,
        rule: &rule,
        binding: &binding,
        closure: Some(crate::flow::Closure {
            state: &endpoint,
            flow: &flow,
            capture: 2,
        }),
    });
    assert_eq!(result.state.frame.len(), 3);
    assert_eq!(result.state.frame[2].particle.len(), 1);
    let imported = &result.state.frame[2].particle[0];
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
