use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use photonic::laser::Laser;
use photonic::runtime::{Limit, Runtime};
use rayon::prelude::*;
use serde::Serialize;

mod corpus;
mod directory;

#[derive(Parser)]
struct Argument {
    #[arg(
        long,
        help = "The repository, for the webbook, language tests and references"
    )]
    root: PathBuf,
    #[arg(long, help = "The optimized bazel-bin holding assembled test programs")]
    bin: PathBuf,
    #[arg(long, default_value_t = 2_000_000, help = "Interpreter work steps")]
    budget: usize,
    #[arg(long, default_value_t = 200_000_000, help = "Laser work steps")]
    allowance: usize,
    #[arg(long, help = "Check only programs whose name contains this text")]
    filter: Option<String>,
    #[arg(
        long,
        default_value_t = 1,
        help = "Laser runs per program, for profiling"
    )]
    repeat: usize,
}

#[derive(Serialize)]
struct Outcome {
    name: String,
    group: String,
    closed: bool,
    verdict: Option<String>,
    state: usize,
    event: usize,
    inferred: usize,
    interpreter: f64,
    laser: f64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    directory::enter()?;
    let argument = Argument::parse();
    let entry = corpus::gather(&argument.root, &argument.bin)
        .into_iter()
        .filter(|entry| {
            argument
                .filter
                .as_ref()
                .is_none_or(|filter| entry.name.contains(filter.as_str()))
        })
        .collect::<Vec<_>>();
    let limit = Limit::default();
    let outcome = entry
        .par_iter()
        .map(|entry| {
            let start = Instant::now();
            let mut runtime = Runtime::new(&entry.program);
            runtime.run(argument.budget, limit);
            let interpreter = start.elapsed().as_secs_f64();
            if !runtime.closed() {
                return Outcome {
                    name: entry.name.clone(),
                    group: entry.group.clone(),
                    closed: false,
                    verdict: None,
                    state: 0,
                    event: 0,
                    inferred: 0,
                    interpreter,
                    laser: 0.0,
                };
            }
            let start = Instant::now();
            let mut laser = Laser::new(&entry.program);
            laser.run(argument.allowance, limit);
            for _ in 1..argument.repeat {
                let mut again = Laser::new(&entry.program);
                again.run(argument.allowance, limit);
            }
            let second = start.elapsed().as_secs_f64() / argument.repeat as f64;
            let summary = laser.summary();
            let verdict = match laser.agree(&runtime) {
                Ok(()) => "agree".to_owned(),
                Err(disagreement) => format!("{disagreement:?}"),
            };
            Outcome {
                name: entry.name.clone(),
                group: entry.group.clone(),
                closed: true,
                verdict: Some(verdict),
                state: summary.state,
                event: summary.event,
                inferred: summary.inferred,
                interpreter,
                laser: second,
            }
        })
        .collect::<Vec<_>>();
    let closed = outcome.iter().filter(|outcome| outcome.closed).count();
    let agree = outcome
        .iter()
        .filter(|outcome| outcome.verdict.as_deref() == Some("agree"))
        .count();
    eprintln!(
        "{} programs, {closed} closed on the interpreter, {agree} agree",
        outcome.len()
    );
    for outcome in outcome
        .iter()
        .filter(|outcome| outcome.closed && outcome.verdict.as_deref() != Some("agree"))
    {
        eprintln!(
            "disagree {}: {}",
            outcome.name,
            outcome.verdict.as_deref().unwrap_or_default()
        );
    }
    println!("{}", serde_json::to_string_pretty(&outcome)?);
    Ok(())
}
