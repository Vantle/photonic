use clap::Parser;
use photonic::path::Search;
use photonic::prism::Outcome;
use photonic::runtime::Limit;
use photonic::source::Program;
use serde::Serialize;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
struct Argument {
    program: PathBuf,
    target: PathBuf,
    #[arg(long, default_value_t = 5)]
    sample: usize,
    #[arg(long, default_value_t = 65_536)]
    state: usize,
}

#[derive(Serialize)]
struct Measurement {
    initialization: f64,
    execution: f64,
    summary: f64,
    release: f64,
    event: usize,
    work: usize,
    statistic: photonic::path::Statistic,
}

fn evaluate(program: Program, target: Program, state: usize) -> Measurement {
    let start = Instant::now();
    let mut search = Search::new(program, target);
    let initialization = start.elapsed().as_secs_f64();
    let start = Instant::now();
    search.run(
        100_000_000,
        Limit {
            state,
            record: 100_000_000,
            world: 1_024,
            cell: 16_384,
            frame: 2_048,
        },
    );
    let execution = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let summary = search.summary();
    let duration = start.elapsed().as_secs_f64();
    assert_eq!(summary.outcome, Outcome::Reached);
    let statistic = search.statistic();
    let start = Instant::now();
    drop(search);
    Measurement {
        initialization,
        execution,
        summary: duration,
        release: start.elapsed().as_secs_f64(),
        event: summary.event,
        work: summary.work,
        statistic,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let program = serde_json::from_str::<Program>(&std::fs::read_to_string(argument.program)?)?;
    let mut target = photonic::lowering::parse(&std::fs::read_to_string(argument.target)?)?;
    target.rule.extend(program.rule.iter().cloned());
    evaluate(program.clone(), target.clone(), argument.state);
    let measurement = (0..argument.sample)
        .map(|_| evaluate(program.clone(), target.clone(), argument.state))
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(std::io::stdout().lock(), &measurement)?;
    Ok(())
}
