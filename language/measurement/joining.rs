use crate::change::Change;
use crate::index::Index;
use crate::joining::Join;
use crate::program::Program;
use crate::state::State;
use serde::Serialize;
use std::hint::black_box;
use std::sync::Arc;
use std::task::Poll;
use std::time::{Duration, Instant};

#[derive(Serialize)]
pub struct Measurement {
    width: usize,
    changing: bool,
    repetition: usize,
    sample: Vec<Sample>,
}

#[derive(Serialize)]
struct Sample {
    execution: f64,
    work: usize,
    binding: usize,
    peak: usize,
}

fn evaluate(program: &Program, state: &[(Arc<State>, Change)]) -> Sample {
    let mut index = Index::new(Arc::new(State::initial(program)));
    let input = crate::plan::Input::shared(&program.rule[0].input, &mut Default::default());
    let store = Arc::new(crate::joining::Store::new(65536));
    let mut join = Join::planned(crate::joining::Request {
        input: &input,
        index: &index,
        frame: 0,
        owner: 0,
        store: &store,
    });
    let mut work = 0;
    let mut binding = 0;
    let mut peak = join.retained();
    let start = Instant::now();
    for (state, change) in state {
        index.update(state.clone(), change);
        join.update(&index);
        join.reset(&index);
        loop {
            work += 1;
            match black_box(join.step(&index)) {
                Poll::Ready(None) => break,
                Poll::Ready(Some(_)) => binding += 1,
                Poll::Pending => {}
            }
        }
        peak = peak.max(join.retained());
    }
    Sample {
        execution: start.elapsed().as_secs_f64(),
        work,
        binding,
        peak,
    }
}

pub fn run() -> Vec<Measurement> {
    let mut report = Vec::new();
    for width in [4, 8, 12] {
        for changing in [false, true] {
            let particle = ["A"; 8].join(".");
            let content = vec!["B.C.E"; width].join(",");
            let pattern = vec!["B"; width - 1]
                .into_iter()
                .chain(["B.E.E"])
                .collect::<Vec<_>>()
                .join(",");
            let source = format!("{particle},{content},C [{particle},{pattern},C] Never");
            let program = Program::new(&crate::lowering::parse(&source).unwrap());
            let mut previous = State::initial(&program);
            let repetition = 64;
            let state = (0..repetition)
                .map(|_| {
                    let removed = if changing {
                        0
                    } else {
                        previous.world.len() - 1
                    };
                    let world = previous.world.remove(removed);
                    previous.world.push(world);
                    let length = previous.world.len();
                    (
                        Arc::new(previous.clone()),
                        Change {
                            world: crate::basis::Set::single(removed),
                            insertion: length - 1..length,
                            frame: Vec::new(),
                        },
                    )
                })
                .collect::<Vec<_>>();
            let start = Instant::now();
            while start.elapsed() < Duration::from_millis(100) {
                black_box(evaluate(&program, &state));
            }
            let sample = (0..7)
                .map(|_| evaluate(&program, &state))
                .collect::<Vec<_>>();
            assert!(sample.iter().all(|sample| sample.binding == 0));
            report.push(Measurement {
                width,
                changing,
                repetition,
                sample,
            });
        }
    }
    report
}
