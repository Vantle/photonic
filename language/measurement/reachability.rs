use crate::change::Change;
use crate::reachability::Index;
use crate::state::{Frame, State, World};
use serde::Serialize;
use std::hint::black_box;
use std::time::{Duration, Instant};

#[derive(Serialize)]
pub struct Measurement {
    world: usize,
    frame: usize,
    change: usize,
    changing: bool,
    sample: Vec<f64>,
}

fn evaluate(initial: &State, state: &[State], change: &Change) -> f64 {
    let mut index = Index::new(initial);
    let mut source = initial;
    let start = Instant::now();
    for state in state {
        index = index.advance(black_box(source), black_box(state), black_box(change));
        source = state;
    }
    let duration = start.elapsed().as_secs_f64();
    assert_eq!(*index.frame, source.reachable());
    duration
}

pub fn run() -> Vec<Measurement> {
    [16, 4096, 65536]
        .into_iter()
        .flat_map(|width| {
            [false, true]
                .into_iter()
                .map(move |changing| (width, changing))
        })
        .map(|(width, changing)| {
            let initial = State {
                world: (0..width)
                    .map(|position| {
                        World {
                            frame: if position + 1 == width { 127 } else { 0 },
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
                            lexical: (frame > 0)
                                .then(|| if frame % 2 == 0 { frame - 1 } else { frame + 1 }),
                            particle: Default::default(),
                            held: Vec::new(),
                        }
                        .into()
                    })
                    .collect(),
            };
            let mut state = Vec::new();
            let mut previous = initial.clone();
            for iteration in 0..128 {
                let mut next = previous.clone();
                next.world.remove(width - 1);
                next.world.push(
                    World {
                        frame: if changing {
                            iteration % 64 * 2 + 1
                        } else {
                            127
                        },
                        particle: Vec::new(),
                    }
                    .into(),
                );
                state.push(next.clone());
                previous = next;
            }
            let change = Change {
                world: [width - 1].into_iter().collect(),
                insertion: width - 1..width,
                frame: Vec::new(),
            };
            let warm = Instant::now();
            while warm.elapsed() < Duration::from_millis(100) {
                evaluate(&initial, &state, &change);
            }
            Measurement {
                world: width,
                frame: initial.frame.len(),
                change: state.len(),
                changing,
                sample: (0..7)
                    .map(|_| evaluate(&initial, &state, &change))
                    .collect(),
            }
        })
        .collect()
}
