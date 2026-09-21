use crate::index::Index;
use crate::program::{Program, Symbol};
use crate::search::Search;
use crate::selection::Store;
use crate::state::{State, Token};
use crate::term::Term;
use serde::Serialize;
use std::sync::Arc;
use std::task::Poll;
use std::time::Instant;

#[derive(Serialize)]
pub struct Measurement {
    execution: f64,
    work: usize,
    binding: usize,
    retained: usize,
}

pub fn run(width: usize, length: usize, shared: bool) -> Measurement {
    let program = Program::new(crate::lowering::parse("A.B.C.D.E.F.G.H,X,Y").unwrap());
    let mut state = State::initial(&program);
    Arc::make_mut(&mut state.world[0])
        .particle
        .extend((0..width).map(|id| Token {
            id: 100 + id,
            value: Symbol::Atom(100),
            capture: None,
        }));
    let common = state.world[0].particle[..8]
        .iter()
        .map(|token| Term::new(token.value, token.capture))
        .collect::<Vec<_>>();
    let pattern = (1..3)
        .map(|position| {
            vec![
                common.clone(),
                vec![Term::new(state.world[position].particle[0].value, None)],
            ]
        })
        .collect::<Vec<_>>();
    let index = Arc::new(Index::new(Arc::new(state)));
    let store = Arc::new(Store::new(65_536));
    let mut work = 0;
    let mut binding = 0;
    let mut retained = 0;
    let start = Instant::now();
    for iteration in 0..length {
        let store = if shared {
            store.clone()
        } else {
            Arc::new(Store::new(65_536))
        };
        let mut search = Search::shared(pattern[iteration % 2].clone(), index.clone(), 0, &store);
        loop {
            work += 1;
            match search.step() {
                Poll::Ready(None) => break,
                Poll::Ready(Some(selection)) => {
                    assert_eq!(selection.len(), 2);
                    binding += 1;
                }
                Poll::Pending => {}
            }
            retained = retained.max(search.retained() + store.retained());
        }
    }
    assert_eq!(binding, length);
    Measurement {
        execution: start.elapsed().as_secs_f64(),
        work,
        binding,
        retained,
    }
}
