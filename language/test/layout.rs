use super::Layout;
use crate::change::Change;
use crate::program::Symbol;
use crate::state::{Frame, State, Token, World};
use std::sync::Arc;

fn token(id: usize) -> Token {
    Token {
        id,
        value: Symbol::Rule(id % 7),
        capture: None,
    }
}

fn state(width: usize) -> State {
    State {
        world: vec![Arc::new(World {
            frame: 0,
            particle: Vec::new(),
        })]
        .into(),
        frame: vec![Arc::new(Frame {
            scope: 0,
            parent: None,
            lexical: None,
            held: Vec::new(),
            particle: (0..width)
                .map(|position| token((width - position) * 5 + 1))
                .collect(),
        })]
        .into(),
    }
}

fn verify(layout: &Layout, source: &State, state: &State, change: &Change) -> Layout {
    let expected = Layout::new(state);
    let actual = layout.advance(
        source,
        state,
        change,
        crate::reachability::Index::new(state),
    );
    assert_eq!(actual.resource, expected.resource);
    assert_eq!(actual.cell, expected.cell);
    assert_eq!(actual.cell, state.size());
    assert_eq!(actual.reach.frame, expected.reach.frame);
    actual
}

#[test]
fn population() {
    for width in [0, 1, 2, 63, 64, 65, 129, 513] {
        let original = state(width);
        let root = Layout::new(&original);
        let mut state = original.clone();
        let mut layout = Layout::new(&state);
        let change = Change {
            world: Default::default(),
            insertion: 1..1,
            frame: vec![0],
        };
        let mut partial = original.clone();
        Arc::make_mut(&mut partial.frame[0])
            .particle
            .retain(|token| token.id != width * 5 + 1);
        let reduced = verify(&root, &original, &partial, &change);
        let restored = verify(&reduced, &partial, &original, &change);
        assert_eq!(restored.resource, root.resource);
        for iteration in 0..width {
            let previous = state.clone();
            Arc::make_mut(&mut state.frame[0])
                .particle
                .retain(|token| token.id != original.frame[0].particle[iteration].id);
            if iteration % 11 == 0 {
                let frame = Arc::make_mut(&mut state.frame[0]);
                frame.particle = frame.particle.iter().cloned().collect();
                frame.particle.reverse();
            }
            layout = verify(&layout, &previous, &state, &change);
            if iteration % 17 == 0 {
                let restored = verify(&layout, &state, &original, &change);
                assert_eq!(restored.resource, root.resource);
                layout = verify(&restored, &original, &state, &change);
            }
        }
        assert_eq!(layout.resource, 0);
        assert_eq!(root.resource, if width == 0 { 0 } else { width * 5 + 2 });
    }
}

#[test]
fn multiplicity() {
    let mut state = state(3);
    let largest = usize::MAX - 1;
    let frame = Arc::make_mut(&mut state.frame[0]);
    frame.particle.push(token(largest));
    frame.particle.push(token(largest));
    frame.held.push(token(largest));
    Arc::make_mut(&mut state.world[0])
        .particle
        .push(token(largest));
    let mut layout = Layout::new(&state);
    assert_eq!(layout.resource, usize::MAX);
    for iteration in 0..6 {
        let source = state.clone();
        let mut change = Change {
            world: Default::default(),
            insertion: 1..1,
            frame: vec![0],
        };
        let frame = Arc::make_mut(&mut state.frame[0]);
        match iteration {
            0 => {
                let mut removed = false;
                frame.particle.retain(|token| {
                    if token.id != largest || removed {
                        return true;
                    }
                    removed = true;
                    false
                });
            }
            1 => frame.particle.retain(|token| token.id != largest),
            2 => {
                Arc::make_mut(&mut state.world[0]).particle.clear();
                change.world = crate::basis::Set::single(0);
                change.insertion = 0..1;
            }
            3 => frame.held.clear(),
            4 => frame.particle.push(token(9000)),
            _ => frame.particle.clear(),
        }
        layout = verify(&layout, &source, &state, &change);
        assert_eq!(
            layout.resource,
            match iteration {
                0..=2 => usize::MAX,
                3 => 17,
                4 => 9001,
                _ => 0,
            }
        );
    }
}

#[test]
fn retirement() {
    let original = state(3);
    let mut state = original.clone();
    let mut layout = Layout::new(&state);
    let change = Change {
        world: crate::basis::Set::single(0),
        insertion: 0..1,
        frame: vec![1],
    };
    for identity in [5000, 7, 9000] {
        let source = state.clone();
        let mut frame = (*state.frame[0]).clone();
        frame.parent = Some(0);
        frame.lexical = Some(0);
        frame.particle = vec![token(identity)].into();
        state.frame.push(Arc::new(frame));
        Arc::make_mut(&mut state.world[0]).frame = 1;
        layout = verify(&layout, &source, &state, &change);
        assert_eq!(layout.resource, (identity + 1).max(17));
        let source = state.clone();
        state = original.clone();
        layout = verify(&layout, &source, &state, &change);
        assert_eq!(layout.resource, 17);
    }
}

#[test]
fn unreachable() {
    let mut state = state(3);
    let mut frame = (*state.frame[0]).clone();
    frame.particle = vec![token(900)].into();
    state.frame.push(Arc::new(frame));
    let mut layout = Layout::new(&state);
    assert_eq!(*layout.reach.frame, vec![0]);
    assert_eq!(layout.resource, 901);
    let change = Change {
        world: Default::default(),
        insertion: 1..1,
        frame: vec![0],
    };
    let source = state.clone();
    Arc::make_mut(&mut state.frame[0])
        .particle
        .push(token(1000));
    layout = verify(&layout, &source, &state, &change);
    assert_eq!(layout.resource, 1001);
    let source = state.clone();
    Arc::make_mut(&mut state.frame[0])
        .particle
        .retain(|token| token.id != 1000);
    layout = verify(&layout, &source, &state, &change);
    assert_eq!(layout.resource, 901);
}
