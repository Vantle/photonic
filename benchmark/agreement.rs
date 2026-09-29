use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use photonic::laser::Laser;
use photonic::laser::net::{Cycle, Exploration, Net, Unsupported};
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
        help = "Explore only every plain schedule, its reduction and its net, to check them on large programs"
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
// must keep its end configurations and cycles. Every net the host explores is explored again on
// Metal, where there is a device, and must agree number for number, limits included.
#[derive(Serialize)]
struct Outcome {
    name: String,
    group: String,
    interpreter: Option<Engine>,
    laser: Option<Engine>,
    verdict: Option<String>,
    plain: Option<String>,
    reduction: Option<Reduction>,
    net: Option<String>,
    metal: Option<String>,
    state: usize,
    event: usize,
    inferred: usize,
}

// A program's net explored on the host, the reference every other exploration of it answers to.
fn host(
    program: &frontend::source::Program,
    limit: Limit,
) -> Result<(Net, Exploration), Unsupported> {
    let mut net = Net::new(program)?;
    let explored = net.explore(limit, Cycle::Find)?;
    Ok((net, explored))
}

fn metal(
    device: &wave::engine::Engine,
    program: &frontend::source::Program,
    limit: Limit,
    host: &Result<(Net, Exploration), Unsupported>,
) -> String {
    let (theirs, expected) = match host {
        Ok(reference) => reference,
        Err(unsupported) => return format!("{unsupported}"),
    };
    let mut net = match Net::new(program) {
        Ok(net) => net,
        Err(unsupported) => return format!("{unsupported}"),
    };
    match device.explore(&mut net, limit, Cycle::Find) {
        Ok(explored) => match explored.agrees(&net, expected, theirs) {
            Ok(()) => "agrees".to_owned(),
            Err(disagreement) => format!("{disagreement:?}"),
        },
        Err(failure) => format!("{failure}"),
    }
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
    let device = wave::engine::Engine::new()?;
    let outcome = entry
        .par_iter()
        .map(|entry| {
            let mut plain = Laser::plain(&entry.program);
            plain.run(argument.allowance, limit);
            let host = (plain.closed() || device.is_some()).then(|| host(&entry.program, limit));
            let net = host
                .as_ref()
                .filter(|_| plain.closed())
                .map(|host| match host {
                    Err(unsupported) => format!("{unsupported}"),
                    Ok((net, explored)) => match explored.mirrors(net, &plain) {
                        Ok(()) => "mirrors".to_owned(),
                        Err(disagreement) => format!("{disagreement:?}"),
                    },
                });
            let metal = device
                .as_ref()
                .zip(host.as_ref())
                .map(|(device, host)| metal(device, &entry.program, limit, host));
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
                    net,
                    metal,
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
                net,
                metal,
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
    let mirror = outcome
        .iter()
        .filter(|outcome| outcome.net.as_deref() == Some("mirrors"))
        .count();
    let mirrored = outcome
        .iter()
        .filter(|outcome| outcome.net.is_some())
        .count();
    let agreeing = outcome
        .iter()
        .filter(|outcome| outcome.metal.as_deref() == Some("agrees"))
        .count();
    let explored = outcome
        .iter()
        .filter(|outcome| outcome.metal.is_some())
        .count();
    let reduction = format!(
        "{preserve} of {reducible} reduced explorations preserve their plain ones, {smaller} of them smaller; {mirror} of {mirrored} nets mirror their plain explorations; {agreeing} of {explored} nets explore identically on Metal"
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
        if let Some(net) = outcome.net.as_ref().filter(|net| net.as_str() != "mirrors") {
            eprintln!("net {}: {net}", outcome.name);
        }
        if let Some(reduction) = outcome
            .reduction
            .as_ref()
            .filter(|reduction| reduction.verdict != "preserves")
        {
            eprintln!("reduced {}: {}", outcome.name, reduction.verdict);
        }
        if let Some(metal) = outcome
            .metal
            .as_ref()
            .filter(|metal| metal.as_str() != "agrees")
        {
            eprintln!("metal {}: {metal}", outcome.name);
        }
    }
    println!("{}", serde_json::to_string_pretty(&outcome)?);
    Ok(())
}
