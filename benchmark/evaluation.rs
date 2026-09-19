use photonic::path::Search;
use photonic::prism::Outcome;
use photonic::runtime::Limit;
use serde::Serialize;
use std::time::Instant;

#[derive(Serialize)]
pub struct Measurement {
    initialization: f64,
    execution: f64,
    event: usize,
    work: usize,
    statistic: photonic::path::Statistic,
}

pub fn evaluate(
    program: photonic::source::Program,
    target: photonic::source::Program,
) -> Measurement {
    let start = Instant::now();
    let mut search = Search::new(program, target).unwrap();
    let initialization = start.elapsed().as_secs_f64();
    let start = Instant::now();
    search.run(
        100_000_000,
        Limit {
            state: 262_144,
            record: 100_000_000,
            world: 1024,
            cell: 16_384,
            frame: 2048,
        },
    );
    let execution = start.elapsed().as_secs_f64();
    let summary = search.summary();
    let statistic = search.statistic();
    assert_eq!(summary.outcome, Outcome::Reached);
    Measurement {
        initialization,
        execution,
        event: summary.event,
        work: summary.work,
        statistic,
    }
}
