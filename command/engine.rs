use crate::disk::Disk;
use crate::{argument, output};
use frontend::source::Program;
use miette::IntoDiagnostic;
use photonic::executor::Executor;
use photonic::laser::Laser;
use photonic::prism::Outcome;
use photonic::runtime::Runtime;
use serde::Serialize;
use spectrum::failure::{Code, Failure};
use spectrum::listing::Listing;
use spectrum::recording::{Engine, Mode};
use std::path::PathBuf;
use std::process::ExitCode;

// The engine's own commands: lower prints a program, run every configuration an exploration
// reaches, and prism the verdict for an exact target. Their text numbers and writes configurations
// as every question's answer does. With --json, run and prism print the engine's own report instead,
// which runtime verification compares from one commit to the next.

#[derive(Serialize)]
struct Report<'program, Execution> {
    outcome: Outcome,
    witness: Option<usize>,
    program: &'program Program,
    target: &'program Program,
    execution: Execution,
}

fn load(file: &[PathBuf], source: &argument::Source) -> Result<Program, Failure> {
    source.subject(file)?.assemble(&Disk::default())
}

// The engine a run explores with: laser unless asked otherwise. The interpreter explores with
// inference, so plain mode refuses it, and metal keeps no configurations to list.
fn engine(verb: &str, argument: &argument::Run) -> Result<Engine, Failure> {
    match (argument.plain, argument.engine) {
        (_, Some(Engine::Metal)) => Err(Failure::new(
            Code::Request,
            format!(
                "{verb} reports every configuration, and metal keeps only counts, ends and cycles; use interpreter or laser, or ask explore and check with --plain --engine metal"
            ),
        )),
        (true, Some(Engine::Interpreter)) => Err(Failure::new(
            Code::Request,
            "--plain runs on laser; the interpreter explores with inference",
        )),
        (_, engine) => Ok(engine.unwrap_or_default()),
    }
}

fn mode(argument: &argument::Run) -> Mode {
    if argument.plain {
        return Mode::Plain;
    }
    Mode::Exhaustive
}

// Every schedule of plain events on Laser, or every future.
fn laser(argument: &argument::Run, program: &Program) -> Laser {
    if argument.plain {
        return Laser::plain(program);
    }
    Laser::new(program)
}

// Prism's answer as an exit status: success only once the target is reached.
fn code(outcome: Outcome) -> ExitCode {
    if outcome == Outcome::Reached {
        return ExitCode::SUCCESS;
    }
    ExitCode::FAILURE
}

pub fn lower(argument: &argument::Lower) -> miette::Result<ExitCode> {
    let program = match load(&argument.file, &argument.source) {
        Ok(program) => program,
        Err(failure) => return output::fail(&failure),
    };
    output::write(&program, false)?;
    Ok(ExitCode::SUCCESS)
}

pub fn run(argument: &argument::Run) -> miette::Result<ExitCode> {
    let (program, engine) = match (
        load(&argument.file, &argument.source),
        engine("run", argument),
    ) {
        (Ok(program), Ok(engine)) => (program, engine),
        (Err(failure), _) | (_, Err(failure)) => return output::fail(&failure),
    };
    let budget = spectrum::budget::Budget::from(&argument.budget);
    if !argument.json {
        return match Listing::new(&program, mode(argument), engine, budget, None) {
            Ok(listing) => {
                output::print(&listing.text())?;
                Ok(ExitCode::SUCCESS)
            }
            Err(failure) => output::fail(&failure),
        };
    }
    let executor = Executor::new(argument.worker).into_diagnostic()?;
    if engine == Engine::Interpreter {
        let mut runtime = Runtime::new(&program);
        runtime.parallel(&executor, budget.work, budget.limit());
        output::write(&runtime.stream(), argument.compact)?;
        return Ok(ExitCode::SUCCESS);
    }
    let mut laser = laser(argument, &program);
    laser.parallel(&executor, budget.work, budget.limit());
    output::write(&laser.report(), argument.compact)?;
    Ok(ExitCode::SUCCESS)
}

pub fn prism(argument: &argument::Prism) -> miette::Result<ExitCode> {
    let (program, mut target, engine) = match (
        load(&argument.run.file, &argument.run.source),
        load(
            std::slice::from_ref(&argument.target),
            &argument::Source::default(),
        ),
        engine("prism", &argument.run),
    ) {
        (Ok(program), Ok(target), Ok(engine)) => (program, target, engine),
        (Err(failure), _, _) | (_, Err(failure), _) | (_, _, Err(failure)) => {
            return output::fail(&failure);
        }
    };
    if argument.preserve {
        target.preserve(&program);
    }
    let budget = spectrum::budget::Budget::from(&argument.run.budget);
    if !argument.run.json {
        let (mode, goal) = if argument.path {
            (Mode::Path, Some(target.clone()))
        } else {
            (mode(&argument.run), None)
        };
        return match Listing::new(&program, mode, engine, budget, goal) {
            Ok(listing) => {
                let verdict = listing.verdict(&target);
                output::print(&listing.judge(&verdict))?;
                Ok(code(verdict.outcome))
            }
            Err(failure) => output::fail(&failure),
        };
    }
    if argument.path {
        let mut search = photonic::path::Search::new(program, Some(target));
        search.run(budget.work, budget.limit());
        output::write(&search.stream(), argument.run.compact)?;
        return Ok(code(search.summary().outcome));
    }
    let executor = Executor::new(argument.run.worker).into_diagnostic()?;
    if engine == Engine::Interpreter {
        let mut runtime = Runtime::new(&program);
        runtime.parallel(&executor, budget.work, budget.limit());
        let verdict = runtime.verdict(&target);
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
        return Ok(code(verdict.outcome));
    }
    let mut laser = laser(&argument.run, &program);
    laser.parallel(&executor, budget.work, budget.limit());
    let verdict = laser.verdict(&target);
    output::write(
        &Report {
            outcome: verdict.outcome,
            witness: verdict.witness,
            program: &program,
            target: &target,
            execution: &laser.report(),
        },
        argument.run.compact,
    )?;
    Ok(code(verdict.outcome))
}
