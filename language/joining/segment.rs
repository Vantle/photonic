use super::playback::Playback;
use super::slot::Slot;
use super::trace::Trace;
use crate::factor::Budget;
use serde::Serialize;
use std::hint::black_box;
use std::sync::Arc;
use std::task::Poll;
use std::time::{Duration, Instant};

#[derive(Serialize)]
pub struct Measurement {
    width: usize,
    token: usize,
    record: usize,
    length: usize,
    extend: bool,
    sample: Vec<f64>,
}

pub fn run() -> Vec<Measurement> {
    let mut report = Vec::new();
    for (width, token, record) in [
        (2, 8, 1),
        (16, 8, 1),
        (16, 64, 1),
        (64, 16, 1),
        (2, 1, 32),
        (2, 1, 256),
    ] {
        let budget = Arc::new(Budget::new(65536));
        let binding = (0..width)
            .map(|position| Slot {
                site: position,
                position,
                token: (0..token).collect(),
            })
            .collect::<Vec<_>>();
        let mut trace = Trace::new(budget.clone(), 1).unwrap();
        for _ in 0..record {
            assert!(trace.append(&Poll::Ready(Some(binding.clone())), 0.., 4096));
        }
        let prefix = [Slot {
            site: width,
            position: 0,
            token: vec![token],
        }];
        let order = (0..=width).collect::<Vec<_>>();
        for extend in [false, true] {
            let operation = || {
                if extend {
                    let mut parent = Trace::new(budget.clone(), 1).unwrap();
                    assert!(parent.extend(black_box(&trace), black_box(&prefix), 4096));
                    parent
                } else {
                    trace.duplicate(4096).unwrap()
                }
            };
            let value = operation();
            let actual = Playback::default().step(&value, &order, &[]).unwrap();
            let expected = if extend {
                prefix
                    .iter()
                    .chain(&binding)
                    .cloned()
                    .enumerate()
                    .map(|(position, mut slot)| {
                        slot.position = position;
                        slot
                    })
                    .collect()
            } else {
                binding.clone()
            };
            assert_eq!(actual, Poll::Ready(Some(expected)));
            drop(value);
            let length = 32768;
            let evaluate = || {
                let start = Instant::now();
                for _ in 0..length {
                    black_box(operation());
                }
                start.elapsed().as_secs_f64()
            };
            let start = Instant::now();
            while start.elapsed() < Duration::from_millis(100) {
                black_box(evaluate());
            }
            report.push(Measurement {
                width,
                token,
                record,
                length,
                extend,
                sample: (0..9).map(|_| evaluate()).collect(),
            });
        }
    }
    report
}
