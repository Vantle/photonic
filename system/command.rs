#![forbid(unsafe_code)]

#[path = "command/argument.rs"]
mod argument;

use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use miette::{IntoDiagnostic, NamedSource, WrapErr};
use photonic::runtime::{Limit, Runtime};
use photonic::snapshot::Node;
use photonic::source::Program;
use photonic::support::Status;

use argument::{Argument, Execution, Format, Operation};

fn main() -> miette::Result<()> {
    match Argument::parse().operation {
        Operation::Parse { path } => parse(path),
        Operation::Run { path, execution } => run(path, execution),
        Operation::Obsidian {
            path,
            target,
            walk,
            execution,
        } => obsidian(path, target, execution, walk),
    }
}

fn read(path: &Path) -> miette::Result<String> {
    std::fs::read_to_string(path)
        .into_diagnostic()
        .wrap_err_with(|| format!("could not read {}", path.display()))
}

fn parse(path: PathBuf) -> miette::Result<()> {
    let source = read(&path)?;
    match photonic::parser::parse(&source) {
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
            Format::Photonic
        }
    });
    if matches!(format, Format::Json) {
        return serde_json::from_str(&source)
            .into_diagnostic()
            .wrap_err_with(|| format!("invalid JSON program in {}", path.display()));
    }
    match photonic::lowering::parse(&source) {
        Ok(program) => Ok(program),
        Err(failure) => Err(miette::Report::new(failure)
            .with_source_code(NamedSource::new(path.display().to_string(), source))),
    }
}

fn load(path: &Path, execution: &Execution) -> miette::Result<Program> {
    let mut source = program(path, execution.format)?;
    for path in &execution.library {
        let library = program(path, Some(Format::Photonic))?;
        if !library.initial.is_empty() {
            return Err(miette::miette!(
                code = "photonic::library",
                "library {} contains initial coherences; supply declarations only",
                path.display()
            ));
        }
        source.rule.extend(library.rule);
    }
    Ok(source)
}

fn run(path: PathBuf, execution: Execution) -> miette::Result<()> {
    let executor = photonic::executor::Executor::new(execution.worker).into_diagnostic()?;
    let mut runtime = Runtime::new(load(&path, &execution)?);
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

fn obsidian(
    path: PathBuf,
    target: PathBuf,
    execution: Execution,
    walk: bool,
) -> miette::Result<()> {
    let executor = photonic::executor::Executor::new(execution.worker).into_diagnostic()?;
    if walk {
        return trace(path, target, execution);
    }
    let mut search = photonic::obsidian::Search::new(
        load(&path, &execution)?,
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
        photonic::obsidian::Outcome::Reached => "Reached",
        photonic::obsidian::Outcome::Unreachable => "Unreachable",
        photonic::obsidian::Outcome::Unknown => "Unknown",
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

fn trace(path: PathBuf, target: PathBuf, execution: Execution) -> miette::Result<()> {
    let mut search = photonic::path::Search::new(
        load(&path, &execution)?,
        program(&target, execution.format)?,
    )?;
    search.run(execution.step, limit(&execution));
    let report = search.report();
    let mut output = std::io::stdout().lock();
    if execution.json {
        serde_json::to_writer_pretty(&mut output, &report).into_diagnostic()?;
        return writeln!(output).into_diagnostic();
    }
    writeln!(output, "{:?}: one direct execution path", report.outcome).into_diagnostic()?;
    if let Some(witness) = report.witness {
        writeln!(
            output,
            "Witness s{witness}: {}",
            display(&report.state[witness])
        )
        .into_diagnostic()?;
    }
    writeln!(
        output,
        "{} events; {} work steps; alternative paths not exhausted",
        report.event.len(),
        report.work
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
