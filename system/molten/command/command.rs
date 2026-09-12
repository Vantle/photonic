#![forbid(unsafe_code)]

mod argument;

use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use miette::{IntoDiagnostic, NamedSource, WrapErr};
use molten::runtime::{Limit, Runtime};
use molten::snapshot::Node;
use molten::source::Program;
use molten::support::Status;

use argument::{Argument, Execution, Format, Operation};

fn main() -> miette::Result<()> {
    match Argument::parse().operation {
        Operation::Parse { path } => parse(path),
        Operation::Run { path, execution } => run(path, execution),
        Operation::Obsidian {
            path,
            target,
            execution,
        } => obsidian(path, target, execution),
    }
}

fn read(path: &Path) -> miette::Result<String> {
    std::fs::read_to_string(path)
        .into_diagnostic()
        .wrap_err_with(|| format!("could not read {}", path.display()))
}

fn parse(path: PathBuf) -> miette::Result<()> {
    let source = read(&path)?;
    match molten::parser::parse(&source) {
        Ok(tree) => writeln!(std::io::stdout().lock(), "{:#?}", tree.node()).into_diagnostic(),
        Err(failure) => Err(miette::Report::new(failure)
            .with_source_code(NamedSource::new(path.display().to_string(), source))),
    }
}

fn program(path: &Path, format: Option<Format>) -> miette::Result<Program> {
    let source = read(path)?;
    let format = format.unwrap_or_else(|| {
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        {
            Format::Json
        } else {
            Format::Molten
        }
    });
    if matches!(format, Format::Json) {
        return serde_json::from_str(&source)
            .into_diagnostic()
            .wrap_err_with(|| format!("invalid JSON program in {}", path.display()));
    }
    match molten::lowering::parse(&source) {
        Ok(program) => Ok(program),
        Err(failure) => Err(miette::Report::new(failure)
            .with_source_code(NamedSource::new(path.display().to_string(), source))),
    }
}

fn run(path: PathBuf, execution: Execution) -> miette::Result<()> {
    let executor = molten::executor::Executor::new(execution.worker).into_diagnostic()?;
    let mut runtime = Runtime::new(program(&path, execution.format)?);
    runtime.parallel(&executor, execution.step, Some(limit(&execution)));
    let snapshot = runtime.snapshot();
    let mut output = std::io::stdout().lock();
    if execution.json {
        serde_json::to_writer_pretty(&mut output, &snapshot).into_diagnostic()?;
        return writeln!(output).into_diagnostic();
    }
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
            display(node)
        )
        .into_diagnostic()?;
    }
    Ok(())
}

fn limit(execution: &Execution) -> Limit {
    Limit {
        state: execution.state,
        record: execution.record,
        world: execution.coherence,
        cell: execution.cell,
        frame: execution.frame,
    }
}

fn obsidian(path: PathBuf, target: PathBuf, execution: Execution) -> miette::Result<()> {
    let executor = molten::executor::Executor::new(execution.worker).into_diagnostic()?;
    let mut search = molten::obsidian::Search::new(
        program(&path, execution.format)?,
        program(&target, execution.format)?,
    )?;
    search.parallel(&executor, execution.step, Some(limit(&execution)));
    let report = search.report();
    let mut output = std::io::stdout().lock();
    if execution.json {
        serde_json::to_writer_pretty(&mut output, &report).into_diagnostic()?;
        return writeln!(output).into_diagnostic();
    }
    let outcome = match report.outcome {
        molten::obsidian::Outcome::Reached => "Reached",
        molten::obsidian::Outcome::Unreachable => "Unreachable",
        molten::obsidian::Outcome::Unknown => "Unknown",
    };
    writeln!(
        output,
        "{outcome}: exact target configuration under the supplied program"
    )
    .into_diagnostic()?;
    if let Some(witness) = report.witness {
        writeln!(
            output,
            "Witness s{witness}: {}",
            display(&report.execution.state[witness])
        )
        .into_diagnostic()?;
    }
    writeln!(
        output,
        "{} configurations; {} queued, {} deferred; exploration {}",
        report.execution.state.len(),
        report.execution.queued,
        report.execution.deferred,
        if report.execution.closed {
            "closed"
        } else {
            "unfinished"
        },
    )
    .into_diagnostic()
}

fn status(value: Status) -> &'static str {
    match value {
        Status::Supported => "supported",
        Status::Unsupported => "unsupported",
    }
}

fn display(node: &Node) -> String {
    if node.world.is_empty() {
        return "∅".to_owned();
    }
    node.world
        .iter()
        .map(|world| {
            let particle = if world.particle.is_empty() {
                "∅".to_owned()
            } else {
                world
                    .particle
                    .iter()
                    .map(|token| match token.capture {
                        Some(frame) => format!("{}@f{frame}", token.display),
                        None => token.display.clone(),
                    })
                    .collect::<Vec<_>>()
                    .join(".")
            };
            format!("[{particle}]@{}", node.frame[world.frame].scope)
        })
        .collect::<Vec<_>>()
        .join(" ")
}
