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
    pub frontier: usize,
    pub length: usize,
    pub sample: usize,
}

#[derive(Serialize)]
pub struct Measurement {
    width: usize,
    count: usize,
    shared: bool,
    sample: Vec<Sample>,
}

#[derive(Serialize)]
struct Sample {
    execution: f64,
    work: usize,
    binding: usize,
    retained: usize,
}

fn drain(join: &mut Join, index: &Index, limit: usize) -> (usize, usize) {
    let mut work = 0;
    let mut binding = 0;
    for _ in 0..limit {
        work += 1;
        match black_box(join.step(index)) {
            Poll::Ready(None) => return (work, binding),
            Poll::Ready(Some(_)) => binding += 1,
            Poll::Pending => {}
        }
    }
    (work, binding)
}

fn evaluate(program: &Program, shared: bool, demand: Option<&Configuration>) -> Sample {
    let mut state = State::initial(program);
    let mut index = Index::new(Arc::new(state.clone()));
    let store = (0..if shared { 1 } else { program.rule.len() })
        .map(|_| Arc::new(Store::new(65536)))
        .collect::<Vec<_>>();
    let mut fragment = Default::default();
    let input = program
        .rule
        .iter()
        .map(|rule| Input::shared(&rule.input, &mut fragment))
        .collect::<Vec<_>>();
    let mut query = input
        .iter()
        .enumerate()
        .map(|(position, input)| {
            let store = &store[if shared { 0 } else { position }];
            Join::planned(Request {
                input,
                index: &index,
                frame: 0,
                owner: 0,
                store,
            })
        })
        .collect::<Vec<_>>();
    for _ in 0..3 {
        let position = state
            .world
            .iter()
            .position(|world| {
                world.particle.len() > 2
                    && world
                        .particle
                        .iter()
                        .all(|token| matches!(token.value, crate::program::Symbol::Atom(_)))
                    && world.particle.len() == input.len() + 1
                    && program.rule[0]
                        .input
                        .last()
                        .unwrap()
                        .iter()
                        .all(|symbol| world.particle.iter().any(|token| token.value == *symbol))
            })
            .unwrap();
        let world = state.world.remove(position);
        state.world.push(world);
        let length = state.world.len();
        index.update(
            Arc::new(state.clone()),
            &Change {
                world: crate::basis::Set::single(position),
                insertion: length - 1..length,
                frame: Vec::new(),
            },
        );
        for query in &mut query {
            query.update(&index);
            query.reset(&index);
        }
    }
    drain(&mut query[0], &index, usize::MAX);
    query[0].reset(&index);
    drain(
        &mut query[0],
        &index,
        demand.map_or(usize::MAX, |demand| demand.frontier),
    );
    if demand.is_none() {
        query[0].reset(&index);
    }
    let start = Instant::now();
    let mut work = 0;
    let mut binding = 0;
    for query in &mut query[1..] {
        query.reset(&index);
        let (count, result) = drain(
            query,
            &index,
            demand.map_or(usize::MAX, |demand| demand.length),
        );
        work += count;
        binding += result;
    }
    Sample {
        execution: start.elapsed().as_secs_f64(),
        work,
        binding,
        retained: store.iter().map(|store| store.retained()).sum::<usize>()
            + query.iter().map(Join::retained).sum::<usize>(),
    }
}

pub fn run() -> Vec<Measurement> {
    let mut report = Vec::new();
    for width in [4, 8, 12] {
        let count = 32;
        let program = program(width, count);
        for shared in [false, true] {
            let start = Instant::now();
            while start.elapsed() < Duration::from_millis(100) {
                black_box(evaluate(&program, shared, None));
            }
            let sample = (0..9)
                .map(|_| evaluate(&program, shared, None))
                .collect::<Vec<_>>();
            assert!(sample.iter().all(|sample| sample.binding == 0));
            report.push(Measurement {
                width,
                count,
                shared,
                sample,
            });
        }
    }
    report
}

fn program(width: usize, count: usize) -> Program {
    let particle = ["A"; 8].join(".");
    let suffix = (0..count)
        .map(|position| format!("S{position}"))
        .collect::<Vec<_>>()
        .join(".");
    let content = vec![format!("B.E.C.{suffix}"); width].join(",");
    let prefix = vec!["B"; width - 1].join(",");
    let rule = (0..count)
        .map(|position| format!("[{particle},{prefix},B.E.E,C.S{position}] Never"))
        .collect::<Vec<_>>()
        .join(", ");
    let source = format!("{particle},{content},C.{suffix}, {rule}");
    Program::new(&crate::lowering::parse(&source).unwrap())
}

pub fn partial(configuration: Configuration) -> Vec<Measurement> {
    let program = program(configuration.width, configuration.count);
    [false, true]
        .into_iter()
        .map(|shared| {
            let start = Instant::now();
            while start.elapsed() < Duration::from_millis(100) {
                black_box(evaluate(&program, shared, Some(&configuration)));
            }
            let sample = (0..configuration.sample)
                .map(|_| evaluate(&program, shared, Some(&configuration)))
                .collect::<Vec<_>>();
            assert!(sample.iter().all(|sample| sample.binding == 0));
            Measurement {
                width: configuration.width,
                count: configuration.count,
                shared,
                sample,
            }
        })
        .collect()
}
