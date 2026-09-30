use std::fmt;
use std::hint::black_box;
use std::num::NonZeroUsize;
use std::process::ExitCode;
use std::time::Instant;

use clap::Parser;
use photonic::executor::Executor;
use photonic::runtime::{Limit, Runtime};
use serde::Serialize;

mod directory;
mod exit;
mod reference;
mod statistic;
mod warm;

#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value_t = NonZeroUsize::MIN)]
    worker: NonZeroUsize,
    #[arg(long)]
    source: Option<std::path::PathBuf>,
    #[arg(long, default_value_t = 12_000, help = "Interpreter work steps")]
    budget: usize,
    #[arg(long, default_value = "25", help = "Measured runs of each program")]
    sample: NonZeroUsize,
}

#[derive(Debug)]
struct Open {
    name: String,
}

impl fmt::Display for Open {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} did not close within the budget; raise --budget",
            self.name
        )
    }
}

impl std::error::Error for Open {}

#[derive(Serialize)]
struct Measurement {
    name: String,
    worker: usize,
    record: usize,
    peak: usize,
    work: usize,
    state: usize,
    event: usize,
    second: statistic::Spread,
}

fn evaluate(
    name: &str,
    program: &frontend::source::Program,
    executor: &Executor,
    budget: usize,
) -> Result<(photonic::snapshot::Snapshot, f64), Open> {
    let program = program.clone();
    let start = Instant::now();
    let mut runtime = Runtime::new(&black_box(program));
    runtime.parallel(executor, budget, Limit::default());
    let snapshot = black_box(runtime.snapshot());
    let second = start.elapsed().as_secs_f64();
    if !snapshot.closed {
        return Err(Open {
            name: name.to_owned(),
        });
    }
    Ok((snapshot, second))
}

fn main() -> ExitCode {
    exit::code(run(&Argument::parse()))
}

fn run(argument: &Argument) -> Result<(), Box<dyn std::error::Error>> {
    directory::enter()?;
    let executor = Executor::new(argument.worker)?;
    let fixture = if let Some(path) = &argument.source {
        vec![reference::Case {
            name: path.display().to_string(),
            program: frontend::lowering::parse(&std::fs::read_to_string(path)?)?,
            closed: true,
        }]
    } else {
        reference::load()?
    };
    let mut report = Vec::new();
    for case in fixture.into_iter().filter(|case| case.closed) {
        let run = || evaluate(&case.name, &case.program, &executor, argument.budget);
        let (result, _) = run()?;
        warm::warm(|| run().map(drop))?;
        let (_, first) = run()?;
        let mut sample = statistic::Sample::new(first);
        for _ in 1..argument.sample.get() {
            let (_, second) = run()?;
            sample.push(second);
        }
        report.push(Measurement {
            name: case.name,
            worker: argument.worker.get(),
            record: result.record,
            peak: result.peak,
            work: result.work,
            state: result.state.len(),
            event: result.event.len(),
            second: sample.spread(),
        });
    }
    serde_json::to_writer_pretty(std::io::stdout().lock(), &report)?;
    Ok(())
}
