use crate::argument;
use miette::IntoDiagnostic;
use spectrum::budget::Budget;
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
        std::fs::read_to_string(path)
            .map_err(|error| Failure::new(Code::File, format!("{path}: {error}")))
    }
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn subject(file: &[PathBuf], source: &argument::Source) -> Subject {
    Subject {
        file: file.iter().map(|path| text(path)).collect(),
        source: source.source.clone(),
        library: source.library.iter().map(|path| text(path)).collect(),
    }
}

fn recording(program: Subject, flag: &argument::Budget, goal: Option<Goal>) -> Recording {
    let default = Budget::default();
    Recording {
        program: Some(program),
        exploration: None,
        mode: if flag.path {
            Mode::Path
        } else {
            Mode::Exhaustive
        },
        budget: Budget {
            work: flag.work.unwrap_or(default.work),
            configuration: flag.configuration.unwrap_or(default.configuration),
            coherence: flag.coherence.unwrap_or(default.coherence),
            occurrence: flag.occurrence.unwrap_or(default.occurrence),
            scope: flag.scope.unwrap_or(default.scope),
            record: flag.record.unwrap_or(default.record),
        },
        goal,
    }
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
        let exact = flag.exact && matches!(kind, Kind::Reach | Kind::Avoid);
        pattern.iter().map(move |pattern| Claim {
            kind,
            pattern: pattern.clone(),
            exact,
            preserve: flag.preserve && exact,
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

fn report(verb: &str, result: &Result<Answer, Failure>, json: bool) -> miette::Result<ExitCode> {
    let code = if result.as_ref().is_ok_and(Answer::passed) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    };
    if json {
        let envelope = request::envelope(Some(verb), result);
        writeln!(std::io::stdout().lock(), "{envelope}").into_diagnostic()?;
        return Ok(code);
    }
    match result {
        Ok(answer) => writeln!(std::io::stdout().lock(), "{}", answer.text()).into_diagnostic()?,
        Err(failure) => writeln!(
            std::io::stderr().lock(),
            "error[{}]: {failure}",
            failure.code
        )
        .into_diagnostic()?,
    }
    Ok(code)
}

fn answer(request: &Request, json: bool) -> miette::Result<ExitCode> {
    let mut store = Store::default();
    let mut context = Context {
        reader: &Disk,
        store: &mut store,
    };
    report(request.verb(), &request.answer(&mut context), json)
}

pub fn question(verb: &str, question: argument::Question) -> miette::Result<ExitCode> {
    let goal = question.goal.map(|configuration| Goal {
        configuration,
        preserve: question.claim.preserve,
    });
    let recording = recording(
        subject(&question.file, &question.source),
        &question.budget,
        goal,
    );
    let claim = claim(&question.claim);
    let request = if verb == "check" {
        Request::Check(spectrum::check::Request { recording, claim })
    } else {
        Request::Explore(spectrum::explore::Request {
            recording,
            claim,
            limit: 12,
        })
    };
    answer(&request, question.source.json)
}

pub fn select(select: argument::Select) -> miette::Result<ExitCode> {
    let request = Request::Select(spectrum::select::Request {
        recording: recording(subject(&select.file, &select.source), &select.budget, None),
        pattern: select.pattern,
        limit: select.limit,
        offset: select.offset,
    });
    answer(&request, select.source.json)
}

pub fn pointer(verb: &str, pointer: argument::Pointer) -> miette::Result<ExitCode> {
    let (file, handle) = split(&pointer.argument);
    let recording = recording(subject(&file, &pointer.source), &pointer.budget, None);
    let request = match (verb, handle) {
        ("step", handle) => Request::Step(spectrum::step::Request {
            recording,
            handle: handle.unwrap_or_else(|| "s0".to_owned()),
        }),
        (verb, None) => {
            let failure = Failure::new(
                Code::Handle,
                "end the arguments with a handle, such as s11, e12 or s11.o1",
            );
            return report(verb, &Err(failure), pointer.source.json);
        }
        ("inspect", Some(handle)) => {
            Request::Inspect(spectrum::inspect::Request { recording, handle })
        }
        (_, Some(handle)) => Request::Cause(spectrum::cause::Request { recording, handle }),
    };
    answer(&request, pointer.source.json)
}

pub fn miss(miss: argument::Miss) -> miette::Result<ExitCode> {
    let (file, rule) = split(&miss.argument);
    let request = Request::Miss(spectrum::miss::Request {
        recording: recording(subject(&file, &miss.source), &miss.budget, None),
        target: miss.target,
        exact: miss.exact,
        preserve: miss.preserve,
        rule,
        limit: miss.limit,
    });
    answer(&request, miss.source.json)
}

pub fn compare(compare: argument::Compare) -> miette::Result<ExitCode> {
    let side = |path: &PathBuf| {
        recording(
            subject(std::slice::from_ref(path), &compare.source),
            &compare.budget,
            None,
        )
    };
    let request = Request::Compare(spectrum::compare::Request {
        left: side(&compare.left),
        right: side(&compare.right),
        claim: claim(&compare.claim),
        limit: compare.limit,
    });
    answer(&request, compare.source.json)
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
    answer(&request, shape.source.json)
}
