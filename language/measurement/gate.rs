use crate::gate::{Gate, Store};
use crate::program::Symbol;
use crate::slot::Slot;
use crate::term::Term;
use serde::Serialize;
use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Serialize)]
pub struct Measurement {
    width: usize,
    shared: bool,
    reused: bool,
    work: usize,
    binding: usize,
    retained: usize,
    sample: Vec<f64>,
}

pub fn run() -> Vec<Measurement> {
    let mut report = Vec::new();
    for width in [16, 128, 512, 2048] {
        let pattern = (0..width)
            .map(|position| vec![Term::new(Symbol::Atom(position), None)])
            .collect::<Vec<_>>();
        for (shared, reused) in [(false, false), (true, false), (true, true)] {
            let evaluate = || {
                let store = Arc::new(Store::new(65536));
                let mut work = 0;
                let mut binding = 0;
                for iteration in 0..128 {
                    if !reused {
                        store.evict();
                    }
                    let mut gate = Gate::new(&pattern);
                    if shared {
                        gate.share(&store);
                    }
                    for position in 0..width {
                        gate.enqueue(Slot {
                            world: position + usize::from(position + 1 == width) * (iteration % 2),
                            position,
                            token: vec![iteration],
                        });
                        while gate.pending() {
                            work += 1;
                            if let Some(result) = gate.step() {
                                assert_eq!(result.len(), width);
                                assert!(result.iter().all(|slot| slot.token == [iteration]));
                                binding += 1;
                                black_box(result);
                            }
                        }
                    }
                }
                (work, binding, store.retained())
            };
            let expected = evaluate();
            let start = Instant::now();
            while start.elapsed() < Duration::from_millis(100) {
                black_box(evaluate());
            }
            let sample = (0..9)
                .map(|_| {
                    let start = Instant::now();
                    let actual = evaluate();
                    let elapsed = start.elapsed().as_secs_f64();
                    assert_eq!(actual, expected);
                    elapsed
                })
                .collect();
            report.push(Measurement {
                width,
                shared,
                reused,
                work: expected.0,
                binding: expected.1,
                retained: expected.2,
                sample,
            });
        }
    }
    report
}
