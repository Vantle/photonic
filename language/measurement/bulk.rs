use crate::canonical::Search;
use crate::executor::Executor;
use crate::program::Symbol;
use crate::state::{Frame, State, Token, World};
use crate::work::{Result, Work};
use serde::Serialize;
use std::sync::Arc;
use std::time::Instant;

#[derive(Serialize)]
pub struct Measurement {
    width: usize,
    worker: usize,
    admitted: bool,
    sample: Vec<f64>,
}

pub fn run() -> Vec<Measurement> {
    let mut report = Vec::new();
    for width in [16, 256, 4096] {
        let state = Arc::new(State {
            world: (0..width)
                .map(|world| {
                    Arc::new(World {
                        frame: 0,
                        particle: vec![Token {
                            id: world,
                            value: Symbol::Atom(world),
                            capture: None,
                        }],
                    })
                })
                .collect(),
            frame: vec![Arc::new(Frame {
                scope: 0,
                parent: None,
                lexical: None,
                particle: Default::default(),
                held: Vec::new(),
            })]
            .into(),
        });
        let expected = state.canonical().state;
        for (worker, admitted) in [(1, false), (2, false), (4, false), (4, true)] {
            let executor = Executor::new(worker).unwrap();
            let evaluate = || {
                let mut batch = (0..32)
                    .map(|position| Work::Normalize(position, Search::new(state.clone())))
                    .collect::<Vec<_>>();
                let mut elapsed = 0.0;
                for step in 0..3 {
                    let start = Instant::now();
                    let result = if !admitted || Work::parallel(&batch) {
                        executor.map(batch, Work::advance)
                    } else {
                        batch.into_iter().map(Work::advance).collect()
                    };
                    elapsed += start.elapsed().as_secs_f64();
                    batch = Vec::new();
                    for (position, result) in result.into_iter().enumerate() {
                        let Result::Normalize(index, search, complete) = result else {
                            unreachable!()
                        };
                        assert_eq!(index, position);
                        assert_eq!(complete, step == 2);
                        if complete {
                            assert_eq!(search.finish().unwrap().state, expected);
                        } else {
                            batch.push(Work::Normalize(index, search));
                        }
                    }
                }
                elapsed
            };
            evaluate();
            report.push(Measurement {
                width,
                worker,
                admitted,
                sample: (0..9).map(|_| evaluate()).collect(),
            });
        }
    }
    report
}
