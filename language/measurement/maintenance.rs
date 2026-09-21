use crate::change::Change;
use crate::index::Index;
use crate::joining::{Join, Request, Store};
use crate::plan::Input;
use crate::program::Program;
use crate::state::State;
use serde::Serialize;
use std::hint::black_box;
use std::sync::Arc;
use std::task::Poll;
use std::time::Instant;

#[derive(Serialize)]
pub struct Measurement {
    execution: f64,
    work: usize,
    pub binding: usize,
    peak: usize,
}

pub(super) fn run<const SCALAR: bool>(
    program: &Program,
    state: &[(Arc<State>, Change)],
) -> Measurement {
    let mut index = Index::new(Arc::new(State::initial(program)));
    let input = Input::shared(&program.rule[0].input, &mut Default::default());
    let store = Arc::new(Store::new(65536));
    let mut join = Join::planned(Request {
        input: &input,
        index: &index,
        frame: 0,
        owner: 0,
        store: &store,
    });
    let mut work = 0;
    let mut binding = 0;
    let mut peak = join.retained() + store.retained();
    let start = Instant::now();
    for (state, change) in state {
        index.update(state.clone(), change);
        join.update(&index);
        join.reset(&index);
        loop {
            if !SCALAR {
                work += join.skip(usize::MAX);
            }
            work += 1;
            match black_box(join.step(&index)) {
                Poll::Ready(None) => break,
                Poll::Ready(Some(_)) => binding += 1,
                Poll::Pending => {}
            }
        }
        peak = peak.max(join.retained() + store.retained());
    }
    Measurement {
        execution: start.elapsed().as_secs_f64(),
        work,
        binding,
        peak,
    }
}
