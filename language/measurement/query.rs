use crate::index::Index;
use crate::program::Symbol;
use crate::state::{Frame, State, Token, World};
use crate::term::Term;
use serde::Serialize;
use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Serialize)]
pub struct Measurement {
    width: usize,
    term: usize,
    candidate: usize,
    initialization: f64,
    repetition: usize,
    sample: Vec<f64>,
}

pub fn run() -> Vec<Measurement> {
    let mut report = Vec::new();
    for width in [128, 1024, 16384] {
        let state = State {
            world: (0..width)
                .map(|world| {
                    World {
                        frame: 0,
                        particle: vec![
                            Token {
                                id: world * 2,
                                value: Symbol::Atom(0),
                                capture: None,
                            },
                            Token {
                                id: world * 2 + 1,
                                value: Symbol::Atom(1 + world % 2),
                                capture: None,
                            },
                        ],
                    }
                    .into()
                })
                .collect(),
            frame: vec![
                Frame {
                    scope: 0,
                    parent: None,
                    lexical: None,
                    held: Vec::new(),
                }
                .into(),
            ]
            .into(),
        };
        let start = Instant::now();
        let index = Index::new(Arc::new(state));
        let initialization = start.elapsed().as_secs_f64();
        for term in 1..=2 {
            let pattern = (0..term)
                .map(|atom| Term::new(Symbol::Atom(atom), None))
                .collect::<Vec<_>>();
            let expected = (0..width)
                .filter(|&world| term == 1 || world % 2 == 0)
                .collect::<Vec<_>>();
            assert_eq!(index.candidate(&pattern, 0), expected);
            let warm = Instant::now();
            while warm.elapsed() < Duration::from_millis(100) {
                black_box(index.candidate(black_box(&pattern), 0));
            }
            let repetition = 10;
            let sample = (0..7)
                .map(|_| {
                    let start = Instant::now();
                    for _ in 0..repetition {
                        black_box(index.candidate(black_box(&pattern), 0));
                    }
                    start.elapsed().as_secs_f64() / repetition as f64
                })
                .collect();
            report.push(Measurement {
                width,
                term,
                candidate: expected.len(),
                initialization,
                repetition,
                sample,
            });
        }
    }
    report
}
