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
use std::time::{Duration, Instant};

pub struct Configuration {
    pub width: usize,
    pub count: usize,
    pub replacement: usize,
    pub length: usize,
    pub productive: bool,
    pub sample: usize,
}

#[derive(Serialize)]
pub struct Measurement {
    execution: f64,
    work: usize,
    binding: usize,
    peak: usize,
}

fn evaluate(program: &Program, state: &[(Arc<State>, Change)]) -> Measurement {
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

pub fn run(configuration: Configuration) -> Vec<Measurement> {
    assert!(configuration.count > 1 && configuration.count <= configuration.width);
    assert!(configuration.replacement > 0 && configuration.replacement <= configuration.count);
    let particle = ["A"; 8].join(".");
    let content = vec![particle.as_str(); configuration.count].join(",");
    let companion = vec!["B.C.E"; configuration.width].join(",");
    let pattern = vec!["B"; configuration.width - 1].join(",");
    let constraint = if configuration.productive {
        "B.E"
    } else {
        "B.E.E"
    };
    let program = Program::new(
        crate::lowering::parse(&format!(
            "{content},{companion},C [{particle},{pattern},{constraint},C] Done"
        ))
        .unwrap(),
    );
    let mut previous = State::initial(&program);
    let state = (0..configuration.length)
        .map(|_| {
            let removal = previous
                .world
                .iter()
                .enumerate()
                .filter(|(_, world)| world.particle.len() == 8)
                .take(configuration.replacement)
                .map(|(position, _)| position)
                .collect::<crate::basis::Set<_>>();
            let insertion = previous.world.len() - removal.len();
            let mut replacement = Vec::new();
            for &position in removal.iter().rev() {
                replacement.push(previous.world.remove(position));
            }
            for world in replacement.into_iter().rev() {
                previous.world.push(world);
            }
            (
                Arc::new(previous.clone()),
                Change {
                    world: removal,
                    insertion: insertion..previous.world.len(),
                    frame: Vec::new(),
                },
            )
        })
        .collect::<Vec<_>>();
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(100) {
        black_box(evaluate(&program, &state));
    }
    let measurement = (0..configuration.sample)
        .map(|_| evaluate(&program, &state))
        .collect::<Vec<_>>();
    let expected = if configuration.productive {
        configuration.count * configuration.width * configuration.length
    } else {
        0
    };
    assert!(
        measurement
            .iter()
            .all(|measurement| measurement.binding == expected)
    );
    measurement
}
