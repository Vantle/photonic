use photonic::path::Search;
use photonic::prism::Outcome;
use photonic::runtime::Limit;
use serde::Serialize;
use std::time::{Duration, Instant};

#[derive(Serialize)]
pub struct Measurement {
    initialization: f64,
    execution: f64,
    event: usize,
    work: usize,
    statistic: photonic::path::Statistic,
    #[cfg(feature = "measurement")]
    phase: Vec<photonic::measurement::profile::Measurement>,
}

pub fn evaluate(
    program: photonic::source::Program,
    target: photonic::source::Program,
    limit: Option<Limit>,
) -> Measurement {
    let limit = limit.unwrap_or(Limit {
        state: 262_144,
        record: 100_000_000,
        world: 1024,
        cell: 16_384,
        frame: 2048,
    });
    let start = Instant::now();
    let mut search = Search::new(program, target).unwrap();
    let initialization = start.elapsed().as_secs_f64();
    #[cfg(feature = "measurement")]
    photonic::measurement::profile::take();
    let start = Instant::now();
    search.run(100_000_000, limit);
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
        #[cfg(feature = "measurement")]
        phase: photonic::measurement::profile::take(),
    }
}

pub fn warm(
    program: &photonic::source::Program,
    target: &photonic::source::Program,
    limit: Option<Limit>,
) {
    let start = Instant::now();
    loop {
        evaluate(program.clone(), target.clone(), limit);
        if start.elapsed() >= Duration::from_millis(100) {
            return;
        }
    }
}
