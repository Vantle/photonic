use crate::change::Change;
use crate::layout::Layout;
use crate::program::Symbol;
use crate::state::{Frame, State, Token, World};
use serde::Serialize;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

#[derive(Serialize)]
pub struct Measurement {
    batch: usize,
    initialization: f64,
    execution: f64,
    resource: usize,
    cell: usize,
}

pub fn run(width: usize, length: usize, descending: bool, replacement: bool) -> Measurement {
    let original = State {
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
                .map(|id| Token {
                    id,
                    value: Symbol::Rule(id % 17),
                    capture: None,
                })
                .collect(),
        })]
        .into(),
    };
    let mut state = original.clone();
    let transition = (0..length.min(width))
        .map(|position| {
            let identity = if descending {
                width - position - 1
            } else {
                position
            };
            let frame = Arc::make_mut(&mut state.frame[0]);
            frame.particle.retain(|token| token.id != identity);
            if replacement {
                frame.particle = frame.particle.iter().cloned().collect();
            }
            (
                Arc::new(state.clone()),
                crate::reachability::Index::new(&state),
            )
        })
        .collect::<Vec<_>>();
    let mut source = Arc::new(original);
    let change = Change {
        world: Default::default(),
        insertion: 1..1,
        frame: vec![0],
    };
    let batch = 64;
    let mut preparation = Vec::with_capacity(batch);
    let start = Instant::now();
    for _ in 0..batch {
        preparation.push(Layout::new(black_box(&source)));
    }
    let initialization = start.elapsed().as_secs_f64() / batch as f64;
    let mut layout = preparation.pop().unwrap();
    drop(preparation);
    let start = Instant::now();
    for (state, reach) in transition {
        layout = layout.advance(black_box(&source), black_box(&state), &change, reach);
        source = state;
    }
    let execution = start.elapsed().as_secs_f64();
    let expected = Layout::new(&state);
    assert_eq!(layout.resource, expected.resource);
    assert_eq!(layout.cell, expected.cell);
    Measurement {
        batch,
        initialization,
        execution,
        resource: layout.resource,
        cell: layout.cell,
    }
}
