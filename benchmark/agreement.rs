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
    #[arg(long, default_value_t = Limit::default().configuration, help = "Configurations kept")]
    configuration: usize,
    #[arg(long, default_value_t = Limit::default().record, help = "Records each engine retains")]
    record: usize,
    #[arg(long, help = "Check only programs whose name contains this text")]
    filter: Option<String>,
    #[arg(
        long,
        default_value_t = 1,
        help = "Laser runs per program, for profiling"
    )]
    repeat: usize,
    #[arg(
        long,
        help = "Explore only every plain schedule and its reduction, to check the reduction on large programs"
    )]
    reduction: bool,
}

#[derive(Serialize)]
struct Engine {
    closed: bool,
    second: f64,
}

#[derive(Serialize)]
struct Reduction {
    verdict: String,
    plain: usize,
    reduced: usize,
}

// Laser runs on every program, so the census also finds programs that only Laser finishes; the
// engines are compared where both close, where Laser closes its plain exploration must be the part
// of its full one that matched events reach, and where the plain exploration closes the reduced one
// must keep its end configurations and cycles.
#[derive(Serialize)]
struct Outcome {
    name: String,
    group: String,
    interpreter: Option<Engine>,
    laser: Option<Engine>,
    verdict: Option<String>,
    plain: Option<String>,
    reduction: Option<Reduction>,
    state: usize,
    event: usize,
    inferred: usize,
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
    let limit = Limit {
        configuration: argument.configuration,
        record: argument.record,
        ..Limit::default()
    };
    let outcome = entry
        .par_iter()
        .map(|entry| {
            let mut plain = Laser::plain(&entry.program);
            plain.run(argument.allowance, limit);
            let reduction = plain.closed().then(|| {
                let mut reduced = Laser::reduced(&entry.program);
                reduced.run(argument.allowance, limit);
                Reduction {
                    verdict: match reduced.preserves(&plain) {
                        Ok(()) => "preserves".to_owned(),
                        Err(disagreement) => format!("{disagreement:?}"),
                    },
                    plain: plain.summary().state,
                    reduced: reduced.summary().state,
                }
            });
            if argument.reduction {
                let summary = plain.summary();
                return Outcome {
                    name: entry.name.clone(),
                    group: entry.group.clone(),
                    interpreter: None,
                    laser: None,
                    verdict: None,
                    plain: None,
                    reduction,
                    state: summary.state,
                    event: summary.event,
                    inferred: summary.inferred,
                };
            }
            let start = Instant::now();
            let mut runtime = Runtime::new(&entry.program);
            runtime.run(argument.budget, limit);
            let interpreter = Engine {
                closed: runtime.closed(),
                second: start.elapsed().as_secs_f64(),
            };
            let start = Instant::now();
            let mut laser = Laser::new(&entry.program);
            laser.run(argument.allowance, limit);
            for _ in 1..argument.repeat {
                let mut again = Laser::new(&entry.program);
                again.run(argument.allowance, limit);
            }
            let second = start.elapsed().as_secs_f64() / argument.repeat as f64;
            let summary = laser.summary();
            let verdict =
                (interpreter.closed && summary.closed).then(|| match laser.agree(&runtime) {
                    Ok(()) => "agree".to_owned(),
                    Err(disagreement) => format!("{disagreement:?}"),
                });
            let plain = summary.closed.then(|| match plain.within(&laser) {
                Ok(()) => "within".to_owned(),
                Err(disagreement) => format!("{disagreement:?}"),
            });
            Outcome {
                name: entry.name.clone(),
                group: entry.group.clone(),
                interpreter: Some(interpreter),
                laser: Some(Engine {
                    closed: summary.closed,
                    second,
                }),
                verdict,
                plain,
                reduction,
                state: summary.state,
                event: summary.event,
                inferred: summary.inferred,
            }
        })
        .collect::<Vec<_>>();
    let closed = outcome
        .iter()
        .filter(|outcome| {
            outcome
                .interpreter
                .as_ref()
                .is_some_and(|engine| engine.closed)
        })
        .count();
    let agree = outcome
        .iter()
        .filter(|outcome| outcome.verdict.as_deref() == Some("agree"))
        .count();
    let finished = |engine: &Option<Engine>| engine.as_ref().is_some_and(|engine| engine.closed);
    let only = outcome
        .iter()
        .filter(|outcome| finished(&outcome.laser) && !finished(&outcome.interpreter))
        .count();
    let open = outcome
        .iter()
        .filter(|outcome| finished(&outcome.interpreter) && !finished(&outcome.laser))
        .count();
    let within = outcome
        .iter()
        .filter(|outcome| outcome.plain.as_deref() == Some("within"))
        .count();
    let plain = outcome
        .iter()
        .filter(|outcome| outcome.plain.is_some())
        .count();
    let reducible = outcome
        .iter()
        .filter(|outcome| outcome.reduction.is_some())
        .count();
    let preserve = outcome
        .iter()
        .flat_map(|outcome| &outcome.reduction)
        .filter(|reduction| reduction.verdict == "preserves")
        .count();
    let smaller = outcome
        .iter()
        .flat_map(|outcome| &outcome.reduction)
        .filter(|reduction| reduction.reduced < reduction.plain)
        .count();
    let reduction = format!(
        "{preserve} of {reducible} reduced explorations preserve their plain ones, {smaller} of them smaller"
    );
    if argument.reduction {
        eprintln!("{} programs; {reduction}", outcome.len());
    } else {
        eprintln!(
            "{} programs, {closed} closed on the interpreter, {agree} agree; {only} close only on Laser and {open} only on the interpreter; {within} of {plain} plain explorations lie within their full ones; {reduction}",
            outcome.len()
        );
    }
    for outcome in outcome.iter().filter(|outcome| {
        outcome
            .verdict
            .as_deref()
            .is_some_and(|verdict| verdict != "agree")
    }) {
        eprintln!(
            "disagree {}: {}",
            outcome.name,
            outcome.verdict.as_deref().unwrap_or_default()
        );
    }
    for outcome in outcome.iter().filter(|outcome| {
        outcome
            .plain
            .as_deref()
            .is_some_and(|plain| plain != "within")
    }) {
        eprintln!(
            "plain {}: {}",
            outcome.name,
            outcome.plain.as_deref().unwrap_or_default()
        );
    }
    for outcome in &outcome {
        if let Some(reduction) = outcome
            .reduction
            .as_ref()
            .filter(|reduction| reduction.verdict != "preserves")
        {
            eprintln!("reduced {}: {}", outcome.name, reduction.verdict);
        }
    }
    println!("{}", serde_json::to_string_pretty(&outcome)?);
    Ok(())
}
