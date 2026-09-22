use crate::executor::Executor;
use crate::index::Index;
use crate::program::{Program, Symbol};
use crate::search::Search;
use crate::state::{State, Token};
use crate::term::Term;
use crate::work::{Result, Work};
use serde::Serialize;
use std::sync::Arc;
use std::task::Poll;
use std::time::Instant;

#[derive(Serialize)]
pub struct Measurement {
    width: usize,
    arity: usize,
    worker: usize,
    admitted: bool,
    sample: Vec<f64>,
}

pub fn run() -> Vec<Measurement> {
    let mut report = Vec::new();
    for (width, arity) in [32, 4096, 65536]
        .into_iter()
        .flat_map(|width| [1, 8].map(|arity| (width, arity)))
    {
        let program = Program::new(&crate::lowering::parse("A").unwrap());
        let mut state = State::initial(&program);
        Arc::make_mut(&mut state.world[0]).particle = (0..width)
            .map(|id| Token {
                id,
                value: Symbol::Atom(id.min(8)),
                capture: None,
            })
            .collect();
        let index = Arc::new(Index::new(Arc::new(state)));
        let pattern = vec![
            (0..arity)
                .map(|position| Term::new(Symbol::Atom(position), None))
                .collect::<Vec<_>>(),
        ];
        for (worker, admitted) in [(1, false), (2, false), (4, false), (4, true)] {
            let executor = Executor::new(worker).unwrap();
            let evaluate = || {
                let mut batch = (0..32)
                    .map(|position| {
                        Work::Search(position, Search::new(pattern.clone(), index.clone(), 0))
                    })
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
                        let Result::Search(identity, search, progress) = result else {
                            unreachable!()
                        };
                        assert_eq!(identity, position);
                        match (step, progress) {
                            (0, Poll::Ready(Some(binding))) => {
                                assert_eq!(binding.len(), 1);
                                assert_eq!(binding[0].token, (0..arity).collect::<Vec<_>>());
                            }
                            (1, Poll::Pending) | (2, Poll::Ready(None)) => {}
                            _ => panic!("unexpected search progress"),
                        }
                        if step < 2 {
                            batch.push(Work::Search(identity, search));
                        }
                    }
                }
                elapsed
            };
            evaluate();
            report.push(Measurement {
                width,
                arity,
                worker,
                admitted,
                sample: (0..9).map(|_| evaluate()).collect(),
            });
        }
    }
    report
}
