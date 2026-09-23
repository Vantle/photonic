#![cfg_attr(not(feature = "allocation"), forbid(unsafe_code))]

use clap::{Parser, Subcommand};
use std::num::NonZeroUsize;
use std::path::PathBuf;

#[cfg(feature = "allocation")]
mod allocation;
mod export;
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
    #[arg(long, global = true)]
    export: Option<export::Mode>,
    #[arg(long, global = true, requires = "export")]
    writer: bool,
}

#[derive(Subcommand)]
enum Case {
    Expression { input: String, expected: String },
    Direct { source: PathBuf, target: String },
    Exhaustive { source: PathBuf },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let (program, target) = match &argument.case {
        Case::Expression { input, expected } => {
            if input.is_empty() || !input.chars().all(|value| "012+-*/()".contains(value)) {
                return Err("use a nonempty ternary expression".into());
            }
            if expected.is_empty() || !expected.chars().all(|value| "012".contains(value)) {
                return Err("use a nonnegative ternary expected result".into());
            }
            let mut program: photonic::source::Program =
                serde_json::from_str(include_str!(env!("FORMULA")))?;
            let encoded = photonic::lowering::parse(&formula::source(input, expected)?)?;
            program.initial = encoded.initial;
            program.rule.extend(encoded.rule);
            let target = photonic::source::Program {
                rule: program.rule.clone(),
                ..photonic::lowering::parse("Done.Zero")?
            };
            (program, Some(target))
        }
        Case::Direct { source, target } => (
            photonic::lowering::parse(&std::fs::read_to_string(source)?)?,
            Some(photonic::lowering::parse(target)?),
        ),
        Case::Exhaustive { source } => (
            photonic::lowering::parse(&std::fs::read_to_string(source)?)?,
            None,
        ),
    };
    let measure = || match &target {
        Some(target) => evaluate(
            || photonic::path::Search::new(program.clone(), target.clone()),
            &argument,
        ),
        None => evaluate(|| photonic::runtime::Runtime::new(&program), &argument),
    };
    let cold = measure();
    let measurement = (0..argument.sample.get())
        .map(|_| measure())
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

#[derive(serde::Serialize)]
#[serde(untagged)]
enum Record {
    Lifecycle(lifecycle::Record),
    Export(export::Record),
}

fn evaluate<Value: lifecycle::Engine>(
    initialize: impl FnOnce() -> Value,
    argument: &Argument,
) -> Record {
    if let Some(mode) = argument.export {
        return Record::Export(export::measure(
            initialize,
            argument.budget.get(),
            mode,
            argument.writer,
        ));
    }
    Record::Lifecycle(lifecycle::measure(initialize, argument.budget.get()))
}
