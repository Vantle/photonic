use crate::index::Index;
use crate::program::{Program, Symbol};
use crate::search::Search;
use crate::selection::Store;
use crate::state::{State, Token};
use crate::term::Term;
use serde::Serialize;
use std::hint::black_box;
use std::sync::Arc;
use std::task::Poll;
use std::time::Instant;

#[derive(Serialize)]
pub struct Measurement {
    width: usize,
    shared: bool,
    reused: bool,
    work: usize,
    retained: usize,
    sample: Vec<f64>,
}

pub fn run() -> Vec<Measurement> {
    let mut report = Vec::new();
    for width in [8, 32, 128, 512] {
        let program = Program::new(crate::lowering::parse("A").unwrap());
        let mut state = State::initial(&program);
        state.world = (0..=width)
            .map(|position| {
                Arc::new(crate::state::World {
                    frame: 0,
                    particle: vec![Token {
                        id: position,
                        value: Symbol::Atom(position),
                        capture: None,
                    }],
                })
            })
            .collect();
        let pattern = (0..width)
            .map(|position| vec![Term::new(Symbol::Atom(position), None)])
            .collect::<Vec<_>>();
        let mut other = state.clone();
        Arc::make_mut(&mut other.world[width]).particle[0].id += 1;
        let index = [state, other].map(|state| Arc::new(Index::new(Arc::new(state))));
        for (shared, reused) in [(false, false), (true, false), (true, true)] {
            let evaluate = || {
                let mut store = Arc::new(Store::new(65536));
                let start = Instant::now();
                let mut work = 0;
                for iteration in 0..64 {
                    if !reused {
                        store = Arc::new(Store::new(65536));
                    }
                    let mut search = if shared {
                        Search::shared(pattern.clone(), index[iteration % 2].clone(), 0, &store)
                    } else {
                        Search::new(pattern.clone(), index[iteration % 2].clone(), 0)
                    };
                    let mut binding = 0;
                    loop {
                        work += 1;
                        match black_box(search.step()) {
                            Poll::Ready(None) => break,
                            Poll::Ready(Some(value)) => {
                                assert_eq!(value.len(), width);
                                binding += 1;
                            }
                            Poll::Pending => {}
                        }
                    }
                    assert_eq!(binding, 1);
                }
                (start.elapsed().as_secs_f64(), work, store.retained())
            };
            let (_, work, retained) = evaluate();
            report.push(Measurement {
                width,
                shared,
                reused,
                work,
                retained,
                sample: (0..9).map(|_| evaluate().0).collect(),
            });
        }
    }
    report
}
