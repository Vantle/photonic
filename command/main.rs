#![forbid(unsafe_code)]

mod argument;
mod output;
mod shape;

use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use miette::{IntoDiagnostic, NamedSource, WrapErr};
use photonic::runtime::{Limit, Runtime};
use photonic::snapshot::Node;
use photonic::source::Program;
use photonic::status::Status;

use argument::{Argument, Execution, Format, Operation};

fn main() -> miette::Result<()> {
    match Argument::parse().operation {
        Operation::Parse { path } => parse(path),
        Operation::Lower { path, context } => lower(path, context),
        Operation::Run { path, execution } => run(path, execution),
        Operation::Prism {
            path,
            target,
            walk,
            execution,
        } => prism(path, target, execution, walk),
        Operation::Symmetry { path, analysis } => shape::symmetry(path, analysis),
        Operation::Compare { path, analysis } => shape::compare(path, analysis),
        Operation::Form { path, analysis } => shape::form(path, analysis),
    }
}

fn read(path: &Path) -> miette::Result<String> {
    std::fs::read_to_string(path)
        .into_diagnostic()
        .wrap_err_with(|| format!("could not read {}", path.display()))
}

fn lower(path: PathBuf, context: Vec<PathBuf>) -> miette::Result<()> {
    let mut source = program(&path, None)?;
    for path in context {
        source.rule.extend(program(&path, None)?.rule);
    }
    output::write(&source, false)
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
    if matches!(format, Format::Photonic) {
        return photonic::lowering::read(path);
    }
    serde_json::from_str(&read(path)?)
        .into_diagnostic()
        .wrap_err_with(|| format!("invalid JSON program in {}", path.display()))
}

fn load(path: &Path, execution: &Execution) -> miette::Result<Program> {
    let mut source = Program::default();
    for library in &execution.library {
        source.declare(photonic::lowering::read(library)?, library.display())?;
    }
    source.append(program(path, execution.format)?);
    Ok(source)
}

fn run(path: PathBuf, execution: Execution) -> miette::Result<()> {
    let executor = photonic::executor::Executor::new(execution.worker).into_diagnostic()?;
    let mut runtime = Runtime::new(&load(&path, &execution)?);
    runtime.parallel(&executor, execution.step, Some(limit(&execution)));
    if execution.json {
        return output::write(&runtime.view(), execution.compact);
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

fn prism(path: PathBuf, target: PathBuf, execution: Execution, walk: bool) -> miette::Result<()> {
    let executor = photonic::executor::Executor::new(execution.worker).into_diagnostic()?;
    if walk {
        return trace(path, target, execution);
    }
    let mut search = photonic::prism::Search::new(
        load(&path, &execution)?,
        program(&target, execution.format)?,
    );
    search.parallel(&executor, execution.step, Some(limit(&execution)));
    if execution.json {
        return output::write(&search.view(), execution.compact);
    }
    let mut output = std::io::stdout().lock();
    let verdict = search.verdict();
    let snapshot = search.snapshot();
    let outcome = match verdict.outcome {
        photonic::prism::Outcome::Reached => "Reached",
        photonic::prism::Outcome::Unreachable => "Unreachable",
        photonic::prism::Outcome::Unknown => "Unknown",
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
            display(&snapshot.state[witness])
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
    .into_diagnostic()
}

fn trace(path: PathBuf, target: PathBuf, execution: Execution) -> miette::Result<()> {
    let mut search = photonic::path::Search::new(
        load(&path, &execution)?,
        program(&target, execution.format)?,
    );
    search.run(execution.step, limit(&execution));
    if execution.json {
        return output::write(&search.view(), execution.compact);
    }
    let mut output = std::io::stdout().lock();
    let report = search.summary();
    writeln!(output, "{:?}: one direct execution path", report.outcome).into_diagnostic()?;
    if let Some(witness) = &report.witness {
        writeln!(output, "Witness s{}: {}", witness.id, display(witness)).into_diagnostic()?;
    }
    writeln!(
        output,
        "{} events; {} work steps; alternative paths not exhausted",
        report.event, report.work
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
    let value = node
        .world
        .iter()
        .map(|world| {
            let particle = if world.particle.is_empty() {
                "∅".to_owned()
            } else {
                world
                    .particle
                    .iter()
                    .map(token)
                    .collect::<Vec<_>>()
                    .join(".")
            };
            format!("[{particle}]@{}", node.frame[world.frame].scope)
        })
        .chain(
            node.frame
                .iter()
                .filter(|frame| !frame.particle.is_empty())
                .map(|frame| {
                    format!(
                        "{{{}}}@{}",
                        frame
                            .particle
                            .iter()
                            .map(token)
                            .collect::<Vec<_>>()
                            .join("."),
                        frame.scope
                    )
                }),
        )
        .collect::<Vec<_>>()
        .join(" ");
    if value.is_empty() {
        return "∅".to_owned();
    }
    value
}

fn token(token: &photonic::snapshot::Token) -> String {
    match token.capture {
        Some(frame) => format!("{}@f{frame}", token.display),
        None => token.display.to_string(),
    }
}
