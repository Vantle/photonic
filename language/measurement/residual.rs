use crate::index::Index;
use crate::plan::Input;
use crate::program::Symbol;
use crate::state::{Frame, State, Token, World};
use serde::Serialize;
use std::hint::black_box;
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

pub fn run(width: usize, length: usize, productive: bool) -> Measurement {
    let state = State {
        frame: vec![Arc::new(Frame {
            scope: 0,
            parent: None,
            lexical: None,
            particle: Default::default(),
            held: Vec::new(),
        })]
        .into(),
        world: vec![Arc::new(World {
            frame: 0,
            particle: (0..width)
                .map(|id| Token {
                    id,
                    value: Symbol::Atom(usize::from(!productive && id + 1 == width)),
                    capture: None,
                })
                .collect(),
        })]
        .into(),
    };
    let index = Index::new(Arc::new(state));
    let input = Input::shared(&[vec![Symbol::Atom(0); width]], &mut Default::default());
    let context = input.context(0);
    let mut work = 0;
    let mut binding = 0;
    let mut retained = 0;
    let start = Instant::now();
    for _ in 0..length {
        let mut search = context.select(0, black_box(&index), index.site(0), None);
        retained = retained.max(search.retained());
        loop {
            work += 1;
            match black_box(search.step()) {
                Poll::Ready(None) => break,
                Poll::Ready(Some(token)) => {
                    assert_eq!(token.len(), width);
                    binding += 1;
                }
                Poll::Pending => {}
            }
        }
    }
    assert_eq!(binding, if productive { length } else { 0 });
    Measurement {
        execution: start.elapsed().as_secs_f64(),
        work,
        binding,
        retained,
    }
}
