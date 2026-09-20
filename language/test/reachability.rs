use crate::change::Change;
use crate::reachability::Index;
use crate::state::{Frame, State, Token, World};
use std::sync::Arc;

#[test]
fn differential() {
    let mut seed = 29u64;
    let mut next = |bound| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as usize % bound
    };
    let mut state = State {
        world: (0..16)
            .map(|frame| {
                World {
                    frame,
                    particle: Vec::new(),
                }
                .into()
            })
            .collect(),
        frame: (0..300)
            .map(|frame| {
                Frame {
                    scope: frame,
                    parent: (frame != 0).then(|| next(frame)),
                    lexical: (frame != 0).then(|| next(300)),
                    held: Vec::new(),
                }
                .into()
            })
            .collect(),
    };
    let mut index = Index::new(&state);
    for step in 0..1024 {
        let previous = state.clone();
        let removed = next(state.world.len());
        state.world.remove(removed);
        state.world.push(
            World {
                frame: next(state.frame.len()),
                particle: vec![Token {
                    id: step,
                    value: crate::program::Symbol::Rule(0),
                    capture: Some(next(state.frame.len())),
                }],
            }
            .into(),
        );
        let frame = if step % 3 == 0 {
            let frame = next(state.frame.len());
            Arc::make_mut(&mut state.frame[frame]).lexical = Some(next(state.frame.len()));
            vec![frame]
        } else {
            Vec::new()
        };
        let change = Change {
            world: [removed].into_iter().collect(),
            insertion: state.world.len() - 1..state.world.len(),
            frame,
        };
        index = index.advance(&previous, &state, &change);
        assert_eq!(*index.frame, state.reachable());
    }
}

#[test]
fn sharing() {
    let program = crate::program::Program::new(crate::lowering::parse("A,B [A] C").unwrap());
    let state = State::initial(&program);
    let index = Index::new(&state);
    let mut changed = state.clone();
    let first = changed.world.remove(0);
    changed.world.push(first);
    let change = Change {
        world: [0].into_iter().collect(),
        insertion: 1..2,
        frame: Vec::new(),
    };
    let next = index.advance(&state, &changed, &change);
    assert!(Arc::ptr_eq(&index.frame, &next.frame));
}
