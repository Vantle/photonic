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
    phase: Vec<photonic::profile::Measurement>,
}

pub fn evaluate(
    program: frontend::source::Program,
    target: frontend::source::Program,
    limit: Option<Limit>,
) -> Result<Measurement, String> {
    let limit = limit.unwrap_or(Limit {
        configuration: 262_144,
        record: 100_000_000,
        coherence: 1024,
        occurrence: 16_384,
        scope: 2048,
    });
    let start = Instant::now();
    let mut search = Search::new(program, Some(target));
    let initialization = start.elapsed().as_secs_f64();
    #[cfg(feature = "measurement")]
    photonic::profile::take();
    let start = Instant::now();
    search.run(100_000_000, limit);
    let execution = start.elapsed().as_secs_f64();
    let summary = search.summary();
    let statistic = search.statistic();
    if summary.outcome != Outcome::Reached {
        return Err(format!(
            "the search ended {:?} instead of reaching the target",
            summary.outcome
        ));
    }
    Ok(Measurement {
        initialization,
        execution,
        event: summary.length,
        work: summary.work,
        statistic,
        #[cfg(feature = "measurement")]
        phase: photonic::profile::take(),
    })
}

pub fn warm(
    program: &frontend::source::Program,
    target: &frontend::source::Program,
    limit: Option<Limit>,
) -> Result<(), String> {
    let start = Instant::now();
    loop {
        evaluate(program.clone(), target.clone(), limit)?;
        if start.elapsed() >= Duration::from_millis(100) {
            return Ok(());
        }
    }
}
