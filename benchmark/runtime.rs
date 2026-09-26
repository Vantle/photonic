use std::hint::black_box;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use clap::Parser;
use photonic::executor::Executor;
use photonic::runtime::{Limit, Runtime};
use serde::{Deserialize, Serialize};

mod directory;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value_t = NonZeroUsize::MIN)]
    worker: NonZeroUsize,
    #[arg(long)]
    source: Option<std::path::PathBuf>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    program: frontend::source::Program,
    closed: bool,
}

#[derive(Serialize)]
struct Measurement {
    name: String,
    sample: usize,
    worker: usize,
    record: usize,
    peak: usize,
    work: usize,
    state: usize,
    event: usize,
    minimum: f64,
    median: f64,
    maximum: f64,
}

fn evaluate(
    program: frontend::source::Program,
    executor: &Executor,
) -> photonic::snapshot::Snapshot {
    let mut runtime = Runtime::new(&black_box(program));
    runtime.parallel(executor, 12_000, Limit::default());
    black_box(runtime.snapshot())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    directory::enter()?;
    let argument = Argument::parse();
    let executor = Executor::new(argument.worker)?;
    let fixture = if let Some(path) = &argument.source {
        vec![Case {
            name: path.display().to_string(),
            program: frontend::lowering::parse(&std::fs::read_to_string(path)?)?,
            closed: true,
        }]
    } else {
        serde_json::from_str::<Vec<Case>>(include_str!("../language/test/reference.json"))?
    };
    let mut report = Vec::new();
    for case in fixture.into_iter().filter(|case| case.closed) {
        let result = evaluate(case.program.clone(), &executor);
        assert!(result.closed, "{} did not close", case.name);
        let warm = Instant::now();
        while warm.elapsed() < Duration::from_millis(100) {
            assert!(evaluate(case.program.clone(), &executor).closed);
        }
        let mut duration = Vec::new();
        for _ in 0..25 {
            let program = case.program.clone();
            let start = Instant::now();
            let result = evaluate(program, &executor);
            duration.push(start.elapsed().as_secs_f64() * 1_000_000.0);
            assert!(result.closed);
        }
        duration.sort_by(f64::total_cmp);
        report.push(Measurement {
            name: case.name,
            sample: duration.len(),
            worker: argument.worker.get(),
            record: result.record,
            peak: result.peak,
            work: result.work,
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
