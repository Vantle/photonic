use crate::argument;
use miette::IntoDiagnostic;
use spectrum::claim::{Claim, Kind};
use spectrum::context::Context;
use spectrum::failure::{Code, Failure};
use spectrum::handle::Handle;
use spectrum::recording::{Goal, Mode, Recording};
use spectrum::request::{self, Answer, Request};
use spectrum::store::Store;
use spectrum::subject::{Reader, Subject};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub struct Disk;

impl Reader for Disk {
    fn read(&self, path: &str) -> Result<String, Failure> {
        let failure = |error: std::io::Error| Failure::new(Code::File, format!("{path}: {error}"));
        if !std::fs::metadata(path).map_err(failure)?.is_file() {
            return Err(Failure::new(Code::File, format!("{path}: not a file")));
        }
        std::fs::read_to_string(path).map_err(failure)
    }
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub fn subject(file: &[PathBuf], source: &argument::Source) -> Subject {
    Subject {
        file: file.iter().map(|path| text(path)).collect(),
        source: source.source.clone(),
        library: source.library.iter().map(|path| text(path)).collect(),
    }
}

fn recording(
    program: Subject,
    budget: &argument::Budget,
    path: bool,
    goal: Option<Goal>,
) -> Recording {
    Recording {
        program: Some(program),
        exploration: None,
        mode: Some(if path { Mode::Path } else { Mode::Exhaustive }),
        budget: Some(budget.into()),
        goal,
    }
}

fn goal(configuration: Option<String>, preserve: bool) -> Option<Goal> {
    configuration.map(|configuration| Goal {
        configuration,
        preserve,
    })
}

fn claim(flag: &argument::Claim) -> Vec<Claim> {
    [
        (Kind::Reach, &flag.reach),
        (Kind::Avoid, &flag.avoid),
        (Kind::Always, &flag.always),
        (Kind::Inevitable, &flag.inevitable),
        (Kind::Outcome, &flag.outcome),
    ]
    .into_iter()
    .flat_map(|(kind, pattern)| {
        pattern.iter().map(move |pattern| Claim {
            kind,
            pattern: pattern.clone(),
            exact: flag.exact,
            preserve: flag.exact && flag.preserve,
        })
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

pub fn fail(failure: &Failure) -> miette::Result<ExitCode> {
    writeln!(
        std::io::stderr().lock(),
        "error[{}]: {failure}",
        failure.code
    )
    .into_diagnostic()?;
    Ok(ExitCode::FAILURE)
}

fn report(verb: &str, result: &Result<Answer, Failure>, json: bool) -> miette::Result<ExitCode> {
    let code = if result.as_ref().is_ok_and(Answer::passed) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    };
    if json {
        let envelope = request::envelope(verb, result);
        writeln!(std::io::stdout().lock(), "{envelope}").into_diagnostic()?;
        return Ok(code);
    }
    match result {
        Ok(answer) => {
            writeln!(std::io::stdout().lock(), "{}", answer.text()).into_diagnostic()?;
            Ok(code)
        }
        Err(failure) => fail(failure),
    }
}

fn answer(request: &Request, json: bool) -> miette::Result<ExitCode> {
    let mut store = Store::default();
    let mut context = Context {
        reader: &Disk,
        store: &mut store,
    };
    report(request.verb(), &request.answer(&mut context), json)
}

fn pointer(
    verb: &str,
    pointer: &argument::Pointer,
    build: impl FnOnce(Recording, String) -> Request,
) -> miette::Result<ExitCode> {
    let (file, handle) = split(&pointer.argument);
    let recording = recording(
        subject(&file, &pointer.source),
        &pointer.budget,
        pointer.path,
        None,
    );
    let Some(handle) = handle else {
        let failure = Failure::new(
            Code::Handle,
            "end the arguments with a handle, such as s11, e12 or s11.o1",
        );
        return report(verb, &Err(failure), pointer.print.json);
    };
    answer(&build(recording, handle), pointer.print.json)
}

pub fn check(check: argument::Check) -> miette::Result<ExitCode> {
    let request = Request::Check(spectrum::check::Request {
        recording: recording(
            subject(&check.file, &check.source),
            &check.budget,
            check.path,
            goal(check.goal, check.claim.preserve),
        ),
        claim: claim(&check.claim),
    });
    answer(&request, check.print.json)
}

pub fn explore(explore: argument::Explore) -> miette::Result<ExitCode> {
    let request = Request::Explore(spectrum::explore::Request {
        recording: recording(
            subject(&explore.file, &explore.source),
            &explore.budget,
            explore.path,
            goal(explore.goal, explore.preserve),
        ),
        limit: explore.limit,
    });
    answer(&request, explore.print.json)
}

pub fn select(select: argument::Select) -> miette::Result<ExitCode> {
    let request = Request::Select(spectrum::select::Request {
        recording: recording(
            subject(&select.file, &select.source),
            &select.budget,
            select.path,
            None,
        ),
        pattern: select.pattern,
        limit: select.limit,
        offset: select.offset,
    });
    answer(&request, select.print.json)
}

pub fn inspect(argument: argument::Pointer) -> miette::Result<ExitCode> {
    pointer("inspect", &argument, |recording, handle| {
        Request::Inspect(spectrum::inspect::Request { recording, handle })
    })
}

pub fn cause(argument: argument::Pointer) -> miette::Result<ExitCode> {
    pointer("cause", &argument, |recording, handle| {
        Request::Cause(spectrum::cause::Request { recording, handle })
    })
}

pub fn step(pointer: argument::Pointer) -> miette::Result<ExitCode> {
    let (file, handle) = split(&pointer.argument);
    let request = Request::Step(spectrum::step::Request {
        recording: recording(
            subject(&file, &pointer.source),
            &pointer.budget,
            pointer.path,
            None,
        ),
        handle: handle.unwrap_or_else(|| "s0".to_owned()),
    });
    answer(&request, pointer.print.json)
}

pub fn miss(miss: argument::Miss) -> miette::Result<ExitCode> {
    let (file, rule) = split(&miss.argument);
    let request = Request::Miss(spectrum::miss::Request {
        recording: recording(subject(&file, &miss.source), &miss.budget, miss.path, None),
        target: miss.target,
        exact: miss.exact,
        preserve: miss.preserve,
        rule,
        limit: miss.limit,
    });
    answer(&request, miss.print.json)
}

pub fn compare(compare: argument::Compare) -> miette::Result<ExitCode> {
    let side = |path: &PathBuf| {
        recording(
            subject(std::slice::from_ref(path), &compare.source),
            &compare.budget,
            compare.path,
            None,
        )
    };
    let request = Request::Compare(spectrum::compare::Request {
        left: side(&compare.left),
        right: side(&compare.right),
        claim: claim(&compare.claim),
        limit: compare.limit,
    });
    answer(&request, compare.print.json)
}

pub fn shape(shape: argument::Shape) -> miette::Result<ExitCode> {
    let request = Request::Shape(spectrum::shape::Request {
        program: shape
            .file
            .iter()
            .map(|file| subject(std::slice::from_ref(file), &shape.source))
            .collect(),
        target: shape.target,
        fix: shape.fix,
        node: shape.node,
    });
    answer(&request, shape.print.json)
}
