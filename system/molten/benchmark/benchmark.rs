use std::hint::black_box;
use std::time::Instant;

use molten::runtime::{Limit, Runtime};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct Case {
    name: String,
    program: molten::source::Program,
    closed: bool,
}

#[derive(Serialize)]
struct Measurement {
    name: String,
    sample: usize,
    state: usize,
    event: usize,
    minimum: f64,
    median: f64,
    maximum: f64,
}

fn evaluate(program: molten::source::Program) -> molten::snapshot::Snapshot {
    let mut runtime = Runtime::new(black_box(program));
    runtime.run(12_000, Some(Limit::default()));
    black_box(runtime.snapshot())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fixture: Vec<Case> = serde_json::from_str(include_str!("../../../example/reference.json"))?;
    let mut report = Vec::new();
    for case in fixture.into_iter().filter(|case| case.closed) {
        let result = evaluate(case.program.clone());
        assert!(result.closed, "{} did not close", case.name);
        let mut duration = Vec::new();
        for _ in 0..25 {
            let program = case.program.clone();
            let start = Instant::now();
            let result = evaluate(program);
            duration.push(start.elapsed().as_secs_f64() * 1_000_000.0);
            assert!(result.closed);
        }
        duration.sort_by(f64::total_cmp);
        report.push(Measurement {
            name: case.name,
            sample: duration.len(),
            state: result.state.len(),
            event: result.event.len(),
            minimum: duration[0],
            median: duration[duration.len() / 2],
            maximum: duration[duration.len() - 1],
        });
    }
    serde_json::to_writer_pretty(std::io::stdout().lock(), &report)?;
    Ok(())
}
