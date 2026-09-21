use crate::basis::Set;
use crate::flow::{Flow, Place, Store};
use serde::Serialize;
use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Serialize)]
pub struct Measurement {
    width: usize,
    shared: bool,
    reused: bool,
    retained: usize,
    sample: Vec<f64>,
}

fn fixture(width: usize) -> (Arc<Flow>, Arc<Flow>) {
    let parent = Arc::new(Flow {
        resource: (0..width)
            .map(|position| {
                (
                    Place::World(0, position),
                    (0..32).map(|token| Place::World(position, token)).collect(),
                )
            })
            .collect(),
        context: (0..width)
            .map(|position| (position..position + 32).collect())
            .collect(),
        frame: vec![Some(3), None, Some(7)],
    });
    let event = Flow {
        resource: (0..32)
            .map(|position| {
                (
                    Place::World(1, position),
                    (0..width)
                        .map(|token| Place::World(0, token))
                        .collect::<Set<_>>(),
                )
            })
            .collect(),
        context: (0..32).map(|_| (0..width).collect()).collect(),
        frame: vec![Some(2), Some(0), None, Some(1)],
    };
    (parent, Arc::new(event))
}

pub fn run() -> Vec<Measurement> {
    let mut report = Vec::new();
    for width in [4, 32, 128, 512] {
        let (parent, event) = fixture(width);
        let expected = parent.compose(&event);
        for (shared, reused) in [(false, false), (true, false), (true, true)] {
            let evaluate = || {
                let mut store = Store::new(65536);
                let start = Instant::now();
                for _ in 0..128 {
                    if !reused {
                        store = Store::new(65536);
                    }
                    let actual = if shared {
                        store.compose(black_box(&parent), black_box(&event))
                    } else {
                        parent.compose(black_box(&event))
                    };
                    black_box(actual);
                }
                (start.elapsed().as_secs_f64(), store.retained())
            };
            let mut store = Store::new(65536);
            assert_eq!(store.compose(&parent, &event), expected);
            assert_eq!(store.compose(&parent, &event), expected);
            let start = Instant::now();
            while start.elapsed() < Duration::from_millis(100) {
                black_box(evaluate());
            }
            let mut retained = 0;
            let sample = (0..9)
                .map(|_| {
                    let result = evaluate();
                    retained = result.1;
                    result.0
                })
                .collect();
            report.push(Measurement {
                width,
                shared,
                reused,
                retained,
                sample,
            });
        }
    }
    report
}
