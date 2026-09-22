use clap::Parser;
use photonic::runtime::{Limit, Runtime};
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
struct Argument {
    source: PathBuf,
    #[arg(long, default_value_t = 5)]
    sample: usize,
}

#[derive(Serialize)]
struct Measurement {
    initialization: f64,
    execution: f64,
    work: usize,
    record: usize,
    state: usize,
    event: usize,
}

fn evaluate(program: photonic::source::Program) -> Measurement {
    let start = Instant::now();
    let mut runtime = Runtime::new(&program);
    let initialization = start.elapsed().as_secs_f64();
    let start = Instant::now();
    runtime.run(
        100_000_000,
        Some(Limit {
            state: 4096,
            record: 100_000_000,
            world: 512,
            cell: 8192,
            frame: 1024,
        }),
    );
    let execution = start.elapsed().as_secs_f64();
    let snapshot = runtime.snapshot();
    assert!(snapshot.closed);
    Measurement {
        initialization,
        execution,
        work: snapshot.work,
        record: snapshot.record,
        state: snapshot.state.len(),
        event: snapshot.event.len(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let program = photonic::lowering::parse(&std::fs::read_to_string(argument.source)?)?;
    evaluate(program.clone());
    let measurement = (0..argument.sample)
        .map(|_| evaluate(program.clone()))
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(std::io::stdout().lock(), &measurement)?;
    Ok(())
}
