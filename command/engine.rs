use crate::{argument, output, verb};
use frontend::source::Program;
use miette::IntoDiagnostic;
use photonic::executor::Executor;
use photonic::laser::Laser;
use photonic::prism::{Outcome, Verdict};
use photonic::runtime::Runtime;
use photonic::snapshot::{Definition, Node, Token, Value};
use photonic::status::Status;
use serde::Serialize;
use spectrum::failure::{Code, Failure};
use spectrum::recording::Engine;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

// The engine's own commands: lower prints a program, run every configuration an exploration
// reaches, and prism the verdict for an exact target. Runtime verification compares their JSON
// reports from one commit to the next.

#[derive(Serialize)]
struct Report<'program, Execution> {
    outcome: Outcome,
    witness: Option<usize>,
    program: &'program Program,
    target: &'program Program,
    execution: Execution,
}

// What an exploration reached, as both engines report it; the interpreter also tells what it
// queued and deferred.
struct Listing<'report> {
    closed: bool,
    state: &'report [Node],
    event: usize,
    work: usize,
    pending: Option<(usize, usize)>,
    definition: &'report [Definition],
}

impl Listing<'_> {
    fn pending(&self) -> String {
        self.pending
            .map(|(queued, deferred)| format!("; {queued} queued, {deferred} deferred"))
            .unwrap_or_default()
    }
}

fn load(file: &[PathBuf], source: &argument::Source) -> Result<Program, Failure> {
    verb::subject(file, source).assemble(&verb::Disk)
}

// The engine a run explores with: the interpreter unless asked otherwise, and laser in plain mode,
// since the interpreter explores with inference.
fn engine(argument: &argument::Run) -> Result<Engine, Failure> {
    match (argument.plain, argument.engine) {
        (true, Some(Engine::Interpreter)) => Err(Failure::new(
            Code::Request,
            "plain mode runs on laser; the interpreter explores with inference",
        )),
        (true, engine) => Ok(engine.unwrap_or(Engine::Laser)),
        (false, engine) => Ok(engine.unwrap_or_default()),
    }
}

// Every schedule of plain events on Laser, or every future.
fn laser(argument: &argument::Run, program: &Program) -> Laser {
    if argument.plain {
        return Laser::plain(program);
    }
    Laser::new(program)
}

fn refuse(verb: &str) -> Failure {
    Failure::new(
        Code::Request,
        format!(
            "{verb} reports every configuration, and metal keeps only counts, ends and cycles; use interpreter or laser, or ask explore and check with --plain --engine metal"
        ),
    )
}

pub fn lower(argument: &argument::Lower) -> miette::Result<ExitCode> {
    let program = match load(&argument.file, &argument.source) {
        Ok(program) => program,
        Err(failure) => return verb::fail(&failure),
    };
    output::write(&program, false)?;
    Ok(ExitCode::SUCCESS)
}

pub fn run(argument: &argument::Run) -> miette::Result<ExitCode> {
    let program = match load(&argument.file, &argument.source) {
        Ok(program) => program,
        Err(failure) => return verb::fail(&failure),
    };
    let engine = match engine(argument) {
        Ok(engine) => engine,
        Err(failure) => return verb::fail(&failure),
    };
    let budget = spectrum::budget::Budget::from(&argument.budget);
    let executor = Executor::new(argument.worker).into_diagnostic()?;
    match engine {
        Engine::Interpreter => {
            let mut runtime = Runtime::new(&program);
            runtime.parallel(&executor, budget.work, budget.limit());
            if argument.json {
                output::write(&runtime.stream(), argument.compact)?;
                return Ok(ExitCode::SUCCESS);
            }
            let snapshot = runtime.snapshot();
            list(&Listing {
                closed: snapshot.closed,
                state: &snapshot.state,
                event: snapshot.event.len(),
                work: snapshot.work,
                pending: Some((snapshot.queued, snapshot.deferred)),
                definition: &snapshot.definition,
            })?;
        }
        Engine::Laser => {
            let mut laser = laser(argument, &program);
            laser.parallel(&executor, budget.work, budget.limit());
            let report = laser.report();
            if argument.json {
                output::write(&report, argument.compact)?;
                return Ok(ExitCode::SUCCESS);
            }
            list(&Listing {
                closed: report.closed,
                state: &report.state,
                event: report.event.len(),
                work: report.work,
                pending: None,
                definition: &report.definition,
            })?;
        }
        Engine::Metal => return verb::fail(&refuse("run")),
    }
    Ok(ExitCode::SUCCESS)
}

fn list(listing: &Listing<'_>) -> miette::Result<()> {
    let mut output = std::io::stdout().lock();
    writeln!(
        output,
        "{}: {} configurations, {} applications, {} work items{}",
        if listing.closed { "Closed" } else { "Paused" },
        listing.state.len(),
        listing.event,
        listing.work,
        listing.pending(),
    )
    .into_diagnostic()?;
    for node in listing.state {
        writeln!(
            output,
            "s{} {} {}",
            node.id,
            status(node.status),
            display(node, listing.definition)
        )
        .into_diagnostic()?;
    }
    Ok(())
}

pub fn prism(argument: &argument::Prism) -> miette::Result<ExitCode> {
    let (program, target) = match (
        load(&argument.run.file, &argument.run.source),
        load(
            std::slice::from_ref(&argument.target),
            &argument::Source::default(),
        ),
    ) {
        (Ok(program), Ok(target)) => (program, target),
        (Err(failure), _) | (_, Err(failure)) => return verb::fail(&failure),
    };
    let budget = spectrum::budget::Budget::from(&argument.run.budget);
    if argument.path {
        return path(argument, program, target, budget);
    }
    let engine = match engine(&argument.run) {
        Ok(engine) => engine,
        Err(failure) => return verb::fail(&failure),
    };
    let executor = Executor::new(argument.run.worker).into_diagnostic()?;
    let json = argument.run.json;
    match engine {
        Engine::Interpreter => {
            let mut runtime = Runtime::new(&program);
            runtime.parallel(&executor, budget.work, budget.limit());
            let verdict = runtime.verdict(&target);
            if json {
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
            let snapshot = runtime.snapshot();
            judge(
                &verdict,
                &Listing {
                    closed: snapshot.closed,
                    state: &snapshot.state,
                    event: snapshot.event.len(),
                    work: snapshot.work,
                    pending: Some((snapshot.queued, snapshot.deferred)),
                    definition: &snapshot.definition,
                },
            )?;
        }
        Engine::Laser => {
            let mut laser = laser(&argument.run, &program);
            laser.parallel(&executor, budget.work, budget.limit());
            let verdict = laser.verdict(&target);
            let execution = laser.report();
            if json {
                output::write(
                    &Report {
                        outcome: verdict.outcome,
                        witness: verdict.witness,
                        program: &program,
                        target: &target,
                        execution: &execution,
                    },
                    argument.run.compact,
                )?;
                return Ok(ExitCode::SUCCESS);
            }
            judge(
                &verdict,
                &Listing {
                    closed: execution.closed,
                    state: &execution.state,
                    event: execution.event.len(),
                    work: execution.work,
                    pending: None,
                    definition: &execution.definition,
                },
            )?;
        }
        Engine::Metal => return verb::fail(&refuse("prism")),
    }
    Ok(ExitCode::SUCCESS)
}

fn judge(verdict: &Verdict, listing: &Listing<'_>) -> miette::Result<()> {
    let mut output = std::io::stdout().lock();
    writeln!(
        output,
        "{}: exact target configuration under the supplied program",
        answer(verdict.outcome)
    )
    .into_diagnostic()?;
    if let Some(witness) = verdict.witness {
        writeln!(
            output,
            "Witness s{witness}: {}",
            display(&listing.state[witness], listing.definition)
        )
        .into_diagnostic()?;
    }
    writeln!(
        output,
        "{} configurations{}; exploration {}",
        listing.state.len(),
        listing.pending(),
        if listing.closed {
            "closed"
        } else {
            "unfinished"
        },
    )
    .into_diagnostic()?;
    Ok(())
}

fn answer(outcome: Outcome) -> &'static str {
    match outcome {
        Outcome::Reached => "Reached",
        Outcome::Unreachable => "Unreachable",
        Outcome::Unknown => "Unknown",
    }
}

fn path(
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
