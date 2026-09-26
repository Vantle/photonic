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
                    particle: Default::default(),
                    held: Vec::new(),
                }
                .into()
            })
            .collect(),
    };
    for _ in 0..1200 {
        state.world.push(
            World {
                frame: 0,
                particle: Vec::new(),
            }
            .into(),
        );
    }
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
    let program = crate::program::Program::new(&frontend::lowering::parse("A,B, [A] C").unwrap());
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

#[test]
fn reclamation() {
    let mut state = State {
        world: std::iter::repeat_n(0, 512)
            .chain([0, 127, 1])
            .map(|frame| {
                World {
                    frame,
                    particle: Vec::new(),
                }
                .into()
            })
            .collect(),
        frame: (0..128)
            .map(|frame| {
                Frame {
                    scope: 0,
                    parent: None,
                    lexical: (frame > 0 && frame < 127)
                        .then(|| if frame % 2 == 0 { frame - 1 } else { frame + 1 }),
                    particle: Default::default(),
                    held: Vec::new(),
                }
                .into()
            })
            .collect(),
    };
    let mut index = Index::new(&state);
    assert_eq!(*index.frame, state.reachable());
    let mut seed = 317u64;
    let mut next = |bound| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as usize % bound
    };
    for iteration in 0..1024 {
        let source = state.clone();
        let frame = next(126) + 1;
        let target = next(128);
        let value = Arc::make_mut(&mut state.frame[frame]);
        value.lexical = Some(target);
        value.held = vec![Token {
            id: iteration,
            value: crate::program::Symbol::Rule(0),
            capture: Some(next(128)),
        }];
        state.world.remove(514);
        state.world.push(
            World {
                frame,
                particle: Vec::new(),
            }
            .into(),
        );
        let change = Change {
            world: [514].into_iter().collect(),
            insertion: 514..515,
            frame: vec![frame],
        };
        index = index.advance(&source, &state, &change);
        assert_eq!(*index.frame, state.reachable(), "iteration {iteration}");
        state = state.reclaim(&index.frame);
        if state.frame.len() < 128 {
            while state.frame.len() < 128 {
                state.frame.push(
                    Frame {
                        scope: 0,
                        parent: None,
                        lexical: None,
                        particle: Default::default(),
                        held: Vec::new(),
                    }
                    .into(),
                );
            }
        }
    }
    let previous = index.retained();
    let released = index.evict();
    assert!(released > 0);
    assert_eq!(index.retained() + released, previous);
    assert_eq!(index.evict(), 0);
    let mut next = index.advance(
        &state,
        &state,
        &Change {
            world: Default::default(),
            insertion: 515..515,
            frame: vec![1],
        },
    );
    assert_eq!(*next.frame, state.reachable());
    assert_eq!(next.evict(), 0);
}

#[test]
fn cycle() {
    let mut state = State {
        world: (0..1024)
            .map(|position| {
                World {
                    frame: usize::from(position == 1023),
                    particle: Vec::new(),
                }
                .into()
            })
            .collect(),
        frame: (0..129)
            .map(|frame| {
                Frame {
                    scope: 0,
                    parent: None,
                    lexical: match frame {
                        1 => Some(2),
                        2 => Some(1),
                        _ => None,
                    },
                    particle: Default::default(),
                    held: Vec::new(),
                }
                .into()
            })
            .collect(),
    };
    let mut index = Index::new(&state);
    assert_eq!(*index.frame, vec![0, 1, 2]);
    let source = state.clone();
    Arc::make_mut(&mut state.world[1023]).frame = 0;
    let change = Change {
        world: [1023].into_iter().collect(),
        insertion: 1023..1024,
        frame: Vec::new(),
    };
    index = index.advance(&source, &state, &change);
    assert_eq!(*index.frame, vec![0]);
    state = state.reclaim(&index.frame);
    let source = state.clone();
    while state.frame.len() < 129 {
        state.frame.push(
            Frame {
                scope: 0,
                parent: None,
                lexical: None,
                particle: Default::default(),
                held: Vec::new(),
            }
            .into(),
        );
    }
    Arc::make_mut(&mut state.world[1023]).frame = 1;
    Arc::make_mut(&mut state.frame[1]).lexical = Some(3);
    index = index.advance(
        &source,
        &state,
        &Change {
            frame: vec![1, 3],
            ..change
        },
    );
    assert_eq!(*index.frame, vec![0, 1, 3]);
}
