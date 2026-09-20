use crate::index::Index;
use crate::joining::{Join, Request, Store};
use crate::plan::Input;
use crate::program::{Program, Symbol};
use crate::state::{State, Token};
use serde::Serialize;
use std::sync::Arc;
use std::task::Poll;
use std::time::Instant;

#[derive(Serialize)]
pub struct Measurement {
    pub execution: f64,
    pub work: usize,
    pub binding: usize,
    pub retained: usize,
}

pub fn run(width: usize, length: usize, shared: bool) -> Measurement {
    let program =
        Program::new(crate::lowering::parse("A.B.C.D.E.F.G.H,X [A.B.C.D.E.F.G.H,X] End").unwrap());
    let mut state = State::initial(&program);
    let world = state
        .world
        .iter()
        .position(|world| world.particle.len() == 8)
        .unwrap();
    Arc::make_mut(&mut state.world[world])
        .particle
        .extend((0..width).map(|position| Token {
            id: 100 + position,
            value: Symbol::Atom(usize::MAX),
            capture: None,
        }));
    let index = Index::new(Arc::new(state));
    let input = Input::shared(&program.rule[0].input, &mut Default::default());
    let store = Arc::new(Store::new(65_536));
    let mut work = 0;
    let mut binding = 0;
    let mut retained = 0;
    let start = Instant::now();
    for _ in 0..length {
        let store = if shared {
            store.clone()
        } else {
            Arc::new(Store::new(65_536))
        };
        let mut search = Join::planned(Request {
            input: &input,
            index: &index,
            frame: 0,
            owner: 0,
            store: &store,
        });
        loop {
            work += 1;
            match search.step(&index) {
                Poll::Ready(Some(selection)) => {
                    assert_eq!(selection.len(), 2);
                    binding += 1;
                }
                Poll::Ready(None) => break,
                Poll::Pending => {}
            }
        }
        retained = retained.max(search.retained() + store.retained());
    }
    assert_eq!(binding, length);
    Measurement {
        execution: start.elapsed().as_secs_f64(),
        work,
        binding,
        retained,
    }
}
