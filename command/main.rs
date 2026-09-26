#![forbid(unsafe_code)]

mod argument;
mod output;
mod server;
mod verb;

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use frontend::source::Program;
use miette::IntoDiagnostic;
use photonic::executor::Executor;
use photonic::prism::Outcome;
use photonic::runtime::Runtime;
use photonic::snapshot::{Definition, Node, Token, Value};
use photonic::status::Status;
use serde::Serialize;
use spectrum::failure::Failure;

use argument::{Argument, Operation};

#[derive(Serialize)]
struct Report<'program, Execution> {
    outcome: Outcome,
    witness: Option<usize>,
    program: &'program Program,
    target: &'program Program,
    execution: Execution,
}

fn main() -> miette::Result<ExitCode> {
    if let Some(directory) = std::env::var_os("BUILD_WORKING_DIRECTORY") {
        std::env::set_current_dir(directory).into_diagnostic()?;
    }
    match Argument::parse().operation {
        Operation::Lower(argument) => lower(&argument),
        Operation::Run(argument) => run(&argument),
        Operation::Prism(argument) => prism(&argument),
        Operation::Check(argument) => verb::check(argument),
        Operation::Explore(argument) => verb::explore(argument),
        Operation::Select(argument) => verb::select(argument),
        Operation::Inspect(argument) => verb::inspect(argument),
        Operation::Cause(argument) => verb::cause(argument),
        Operation::Miss(argument) => verb::miss(argument),
        Operation::Step(argument) => verb::step(argument),
        Operation::Compare(argument) => verb::compare(argument),
        Operation::Shape(argument) => verb::shape(argument),
        Operation::Mcp => server::serve(),
    }
}

fn load(file: &[PathBuf], source: &argument::Source) -> Result<Program, Failure> {
    verb::subject(file, source).assemble(&verb::Disk)
}

fn lower(argument: &argument::Lower) -> miette::Result<ExitCode> {
    let program = match load(&argument.file, &argument.source) {
        Ok(program) => program,
        Err(failure) => return verb::fail(&failure),
    };
    output::write(&program, false)?;
    Ok(ExitCode::SUCCESS)
}

fn run(argument: &argument::Run) -> miette::Result<ExitCode> {
    let program = match load(&argument.file, &argument.source) {
        Ok(program) => program,
        Err(failure) => return verb::fail(&failure),
    };
    let budget = spectrum::budget::Budget::from(&argument.budget);
    let executor = Executor::new(argument.worker).into_diagnostic()?;
    let mut runtime = Runtime::new(&program);
    runtime.parallel(&executor, budget.work, budget.limit());
    if argument.json {
        output::write(&runtime.stream(), argument.compact)?;
        return Ok(ExitCode::SUCCESS);
    }
    let mut output = std::io::stdout().lock();
    let snapshot = runtime.snapshot();
    writeln!(
        output,
        "{}: {} configurations, {} applications, {} work items; {} queued, {} deferred",
        if snapshot.closed { "Closed" } else { "Paused" },
        snapshot.state.len(),
        snapshot.event.len(),
        snapshot.work,
        snapshot.queued,
        snapshot.deferred,
    )
    .into_diagnostic()?;
    for node in &snapshot.state {
        writeln!(
            output,
            "s{} {} {}",
            node.id,
            status(node.status),
            display(node, &snapshot.definition)
        )
        .into_diagnostic()?;
    }
    Ok(ExitCode::SUCCESS)
}

fn prism(argument: &argument::Prism) -> miette::Result<ExitCode> {
    let (program, target) = match (
        load(&argument.run.file, &argument.run.source),
        load(
            std::slice::from_ref(&argument.target),
            &argument::Source::default(),
        )
        .and_then(spectrum::subject::target),
    ) {
        (Ok(program), Ok(target)) => (program, target),
        (Err(failure), _) | (_, Err(failure)) => return verb::fail(&failure),
    };
    let budget = spectrum::budget::Budget::from(&argument.run.budget);
    if argument.path {
        return walk(argument, program, target, budget);
    }
    let executor = Executor::new(argument.run.worker).into_diagnostic()?;
    let mut runtime = Runtime::new(&program);
    runtime.parallel(&executor, budget.work, budget.limit());
    let verdict = runtime.verdict(&target);
    if argument.run.json {
        output::write(
            &Report {
                outcome: verdict.outcome,
                witness: verdict.witness,
                program: &program,
                target: &target,
                execution: runtime.stream(),
            },
            argument.run.compact,
        )?;
        return Ok(ExitCode::SUCCESS);
    }
    let mut output = std::io::stdout().lock();
    let snapshot = runtime.snapshot();
    let outcome = match verdict.outcome {
        Outcome::Reached => "Reached",
        Outcome::Unreachable => "Unreachable",
        Outcome::Unknown => "Unknown",
    };
    writeln!(
        output,
        "{outcome}: exact target configuration under the supplied program"
    )
    .into_diagnostic()?;
    if let Some(witness) = verdict.witness {
        writeln!(
            output,
            "Witness s{witness}: {}",
            display(&snapshot.state[witness], &snapshot.definition)
        )
        .into_diagnostic()?;
    }
    writeln!(
        output,
        "{} configurations; {} queued, {} deferred; exploration {}",
        snapshot.state.len(),
        snapshot.queued,
        snapshot.deferred,
        if snapshot.closed {
            "closed"
        } else {
            "unfinished"
        },
    )
    .into_diagnostic()?;
    Ok(ExitCode::SUCCESS)
}

fn walk(
    argument: &argument::Prism,
    program: Program,
    target: Program,
    budget: spectrum::budget::Budget,
) -> miette::Result<ExitCode> {
    let mut search = photonic::path::Search::new(program, Some(target));
    search.run(budget.work, budget.limit());
    if argument.run.json {
        output::write(&search.stream(), argument.run.compact)?;
        return Ok(ExitCode::SUCCESS);
    }
    let mut output = std::io::stdout().lock();
    let summary = search.summary();
    writeln!(output, "{:?}: one direct execution path", summary.outcome).into_diagnostic()?;
    if let Some(witness) = &summary.witness {
        writeln!(
            output,
            "Witness s{}: {}",
            witness.id,
            display(witness, &search.definition())
        )
        .into_diagnostic()?;
    }
    writeln!(
        output,
        "{} events; {} work steps; alternative paths not exhausted",
        summary.length, summary.work
    )
    .into_diagnostic()?;
    Ok(ExitCode::SUCCESS)
}

fn status(value: Status) -> &'static str {
    match value {
        Status::Supported => "supported",
        Status::Unsupported => "unsupported",
    }
}

fn display(node: &Node, definition: &[Definition]) -> String {
    let token = |token: &Token| {
        let text = match &token.value {
            Value::Atom(atom) => atom.to_string(),
            Value::Rule(rule) => format!("⟨{}⟩", definition[*rule].name),
        };
        match token.capture {
            Some(frame) => format!("{text}@f{frame}"),
            None => text,
        }
    };
    let particle = |particle: &[Token]| particle.iter().map(token).collect::<Vec<_>>().join(".");
    let value = node
        .world
        .iter()
        .map(|world| {
            let text = if world.particle.is_empty() {
                "∅".to_owned()
            } else {
                particle(&world.particle)
            };
            format!("[{text}]@{}", node.frame[world.frame].scope)
        })
        .chain(
            node.frame
                .iter()
                .filter(|frame| !frame.particle.is_empty())
                .map(|frame| format!("{{{}}}@{}", particle(&frame.particle), frame.scope)),
        )
        .collect::<Vec<_>>()
        .join(" ");
    if value.is_empty() {
        return "∅".to_owned();
    }
    value
}
