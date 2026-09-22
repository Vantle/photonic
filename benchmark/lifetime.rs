#![cfg_attr(not(feature = "allocation"), forbid(unsafe_code))]

use clap::{Parser, Subcommand};
use std::num::NonZeroUsize;
use std::path::PathBuf;

#[cfg(feature = "allocation")]
mod allocation;
mod formula;
mod lifecycle;
mod meter;

#[derive(Parser)]
struct Argument {
    #[command(subcommand)]
    case: Case,
    #[arg(long, global = true, default_value = "3")]
    sample: NonZeroUsize,
    #[arg(long, global = true, default_value = "100000000")]
    budget: NonZeroUsize,
}

#[derive(Subcommand)]
enum Case {
    Expression { input: String, expected: String },
    Direct { source: PathBuf, target: String },
    Exhaustive { source: PathBuf },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let (program, target) = match argument.case {
        Case::Expression { input, expected } => {
            if input.is_empty() || !input.chars().all(|value| "012+-*/()".contains(value)) {
                return Err("use a nonempty ternary expression".into());
            }
            if expected.is_empty() || !expected.chars().all(|value| "012".contains(value)) {
                return Err("use a nonnegative ternary expected result".into());
            }
            let mut program: photonic::source::Program =
                serde_json::from_str(include_str!(env!("FORMULA")))?;
            let encoded = photonic::lowering::parse(&formula::source(&input, &expected))?;
            program.initial = encoded.initial;
            program.rule.extend(encoded.rule);
            (program, Some(photonic::lowering::parse("Done.Zero")?))
        }
        Case::Direct { source, target } => (
            photonic::lowering::parse(&std::fs::read_to_string(source)?)?,
            Some(photonic::lowering::parse(&target)?),
        ),
        Case::Exhaustive { source } => (
            photonic::lowering::parse(&std::fs::read_to_string(source)?)?,
            None,
        ),
    };
    let evaluate = || match &target {
        Some(target) => lifecycle::measure(
            || photonic::path::Search::new(program.clone(), target.clone()).unwrap(),
            argument.budget.get(),
        ),
        None => lifecycle::measure(
            || photonic::runtime::Runtime::new(program.clone()),
            argument.budget.get(),
        ),
    };
    let cold = evaluate();
    let measurement = (0..argument.sample.get())
        .map(|_| evaluate())
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "allocation": cfg!(feature = "allocation"),
            "budget": argument.budget.get(),
            "cold": cold,
            "measurement": measurement,
        }),
    )?;
    Ok(())
}
