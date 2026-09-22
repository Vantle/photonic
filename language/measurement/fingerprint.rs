use crate::change::Change;
use crate::fingerprint::Index;
use crate::layout::Layout;
use crate::program::Symbol;
use crate::state::{Frame, State, Token, World};
use serde::Serialize;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

#[derive(Serialize)]
pub struct Measurement {
    initialization: f64,
    execution: f64,
    fingerprint: u64,
    retained: usize,
    released: usize,
}

pub fn run(width: usize, depth: usize, length: usize, replacement: bool) -> Measurement {
    let original = State {
        world: vec![Arc::new(World {
            frame: 0,
            particle: Vec::new(),
        })]
        .into(),
        frame: (0..depth)
            .map(|frame| {
                Arc::new(Frame {
                    scope: frame % 4,
                    parent: None,
                    lexical: None,
                    held: Vec::new(),
                    particle: if frame == 0 {
                        (0..width)
                            .map(|id| Token {
                                id,
                                value: Symbol::Rule(id % 17),
                                capture: Some(id % depth),
                            })
                            .collect()
                    } else {
                        Default::default()
                    },
                })
            })
            .collect(),
    };
    let mut state = original.clone();
    let transition = (0..length.min(width))
        .map(|position| {
            let frame = Arc::make_mut(&mut state.frame[0]);
            frame.particle.retain(|token| token.id != position);
            if replacement {
                frame.particle = frame.particle.iter().cloned().collect();
            }
            (Arc::new(state.clone()), Layout::new(&state))
        })
        .collect::<Vec<_>>();
    let start = Instant::now();
    let mut index = Index::new(black_box(Arc::new(original)));
    let initialization = start.elapsed().as_secs_f64();
    let change = Change {
        world: Default::default(),
        insertion: 1..1,
        frame: vec![0],
    };
    let start = Instant::now();
    for (state, layout) in transition {
        index = index.advance(black_box(state), &change, layout);
    }
    let execution = start.elapsed().as_secs_f64();
    assert_eq!(index.value, Index::new(Arc::new(state)).value);
    let retained = index.retained();
    let released = index.evict();
    assert_eq!(index.retained(), retained - released);
    Measurement {
        initialization,
        execution,
        fingerprint: index.value,
        retained,
        released,
    }
}
