use crate::disk::Disk;
use crate::{argument, output};
use spectrum::claim::Claim;
use spectrum::context::Context;
use spectrum::failure::{Code, Failure};
use spectrum::handle::Handle;
use spectrum::recording::{Goal, Mode, Recording};
use spectrum::request::{self, Answer, Request};
use spectrum::store::Store;
use std::path::PathBuf;
use std::process::ExitCode;

fn recording(file: &[PathBuf], flag: &argument::Recording) -> Result<Recording, Failure> {
    let mode = match (flag.search.path, flag.search.plain) {
        (true, _) => Mode::Path,
        (false, true) => Mode::Plain,
        (false, false) => Mode::Exhaustive,
    };
    Ok(Recording {
        program: Some(flag.source.subject(file)?),
        exploration: None,
        mode: Some(mode),
        engine: flag.search.engine,
        budget: Some((&flag.budget).into()),
        goal: flag.goal.clone().map(|configuration| Goal {
            configuration,
            preserve: flag.preserve,
        }),
    })
}

fn claim(flag: &argument::Claim, preserve: bool) -> Vec<Claim> {
    flag.pattern
        .iter()
        .map(|(kind, pattern)| Claim {
            kind: *kind,
            pattern: pattern.clone(),
            exact: flag.exact,
            preserve: flag.exact && preserve,
        })
        .collect()
}

fn split(argument: &[String]) -> (Vec<PathBuf>, Option<String>) {
    match argument.split_last() {
        Some((last, rest)) if last.parse::<Handle>().is_ok() => {
            (rest.iter().map(PathBuf::from).collect(), Some(last.clone()))
        }
        _ => (argument.iter().map(PathBuf::from).collect(), None),
    }
}

// The files, then the handle the arguments must end with.
fn pointer(argument: &[String], example: &str) -> Result<(Vec<PathBuf>, String), Failure> {
    match split(argument) {
        (file, Some(handle)) => Ok((file, handle)),
        (_, None) => Err(Failure::new(
            Code::Handle,
            format!("end the arguments with a handle, such as {example}"),
        )),
    }
}

fn report(verb: &str, result: &Result<Answer, Failure>, json: bool) -> miette::Result<ExitCode> {
    let code = if result.as_ref().is_ok_and(Answer::passed) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    };
    if json {
        output::print(&request::envelope(verb, result))?;
        return Ok(code);
    }
    match result {
        Ok(answer) => {
            output::print(&answer.text())?;
            Ok(code)
        }
        Err(failure) => output::fail(failure),
    }
}

fn answer(verb: &str, request: Result<Request, Failure>, json: bool) -> miette::Result<ExitCode> {
    let disk = Disk::default();
    let mut store = Store::default();
    let mut context = Context {
        reader: &disk,
        store: &mut store,
    };
    let result = request.and_then(|request| request.answer(&mut context));
    report(verb, &result, json)
}

pub fn check(check: argument::Check) -> miette::Result<ExitCode> {
    let request = recording(&check.file, &check.recording).map(|recording| {
        Request::Check(spectrum::check::Request {
            recording,
            claim: claim(&check.claim, check.recording.preserve),
        })
    });
    answer("check", request, check.print.json)
}

pub fn explore(explore: argument::Explore) -> miette::Result<ExitCode> {
    let request = recording(&explore.file, &explore.recording).map(|recording| {
        Request::Explore(spectrum::explore::Request {
            recording,
            limit: explore.limit,
        })
    });
    answer("explore", request, explore.print.json)
}

pub fn select(select: argument::Select) -> miette::Result<ExitCode> {
    let request = recording(&select.file, &select.recording).map(|recording| {
        Request::Select(spectrum::select::Request {
            recording,
            pattern: select.pattern,
            limit: select.limit,
            offset: select.offset,
        })
    });
    answer("select", request, select.print.json)
}

pub fn inspect(inspect: argument::Inspect) -> miette::Result<ExitCode> {
    let request = pointer(&inspect.argument, "r2, s11, e12, s11.c0, s11.o1 or s10.f1").and_then(
        |(file, handle)| {
            Ok(Request::Inspect(spectrum::inspect::Request {
                recording: recording(&file, &inspect.recording)?,
                handle,
            }))
        },
    );
    answer("inspect", request, inspect.print.json)
}

pub fn cause(cause: argument::Cause) -> miette::Result<ExitCode> {
    let request = pointer(&cause.argument, "s11, e12 or s11.o1").and_then(|(file, handle)| {
        Ok(Request::Cause(spectrum::cause::Request {
            recording: recording(&file, &cause.recording)?,
            handle,
        }))
    });
    answer("cause", request, cause.print.json)
}

pub fn step(step: argument::Step) -> miette::Result<ExitCode> {
    let (file, handle) = split(&step.argument);
    let request = recording(&file, &step.recording).map(|recording| {
        Request::Step(spectrum::step::Request {
            recording,
            handle: handle.unwrap_or_else(|| "s0".to_owned()),
        })
    });
    answer("step", request, step.print.json)
}

pub fn miss(miss: argument::Miss) -> miette::Result<ExitCode> {
    let (file, rule) = split(&miss.argument);
    let request = recording(&file, &miss.recording).map(|recording| {
        Request::Miss(spectrum::miss::Request {
            recording,
            target: miss.target,
            exact: miss.exact,
            preserve: miss.exact && miss.recording.preserve,
            rule,
            limit: miss.limit,
        })
    });
    answer("miss", request, miss.print.json)
}

pub fn compare(compare: argument::Compare) -> miette::Result<ExitCode> {
    let side = |path: &PathBuf| recording(std::slice::from_ref(path), &compare.recording);
    let request = side(&compare.left).and_then(|left| {
        Ok(Request::Compare(spectrum::compare::Request {
            left,
            right: side(&compare.right)?,
            claim: claim(&compare.claim, compare.recording.preserve),
            limit: compare.limit,
        }))
    });
    answer("compare", request, compare.print.json)
}

// Each file is a program with the inline source added, or the inline source alone is one.
pub fn shape(shape: argument::Shape) -> miette::Result<ExitCode> {
    let program = if shape.file.is_empty() {
        shape.source.subject(&[]).map(|subject| vec![subject])
    } else {
        shape
            .file
            .iter()
            .map(|file| shape.source.subject(std::slice::from_ref(file)))
            .collect()
    };
    let request = program.map(|program| {
        Request::Shape(spectrum::shape::Request {
            program,
            target: shape.target,
            fix: shape.fix,
            node: shape.node,
        })
    });
    answer("shape", request, shape.print.json)
}
