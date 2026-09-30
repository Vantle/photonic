use frontend::source::{self, Program};
use miette::{IntoDiagnostic, WrapErr};
use photonic::laser::Laser;
use photonic::prism::Outcome;
use photonic::runtime::{Limit, Runtime};
use photonic::snapshot::{Definition, Node, Value};
use photonic::status::Status;
use photonic::stop::Stop;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::process::ExitCode;

#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

// A failure prints this many of the configurations it found; the undeclared outputs keep them all.
const SHOWN: usize = 3;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Expect {
    Reached,
    Unreachable,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    program: String,
    source: String,
    target: Vec<String>,
    expect: Expect,
    path: bool,
    every: bool,
    work: usize,
    limit: Limit,
}

// How far an exploration went, and which budget or limit stopped it short of closing. A failure
// prints it, so that a budget that stopped the search is told apart from a program that went wrong.
#[derive(Serialize)]
struct Extent {
    closed: bool,
    configuration: usize,
    event: usize,
    work: usize,
    stop: Vec<Stop>,
}

impl Display for Extent {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let state = if self.closed { "closed" } else { "open" };
        write!(
            formatter,
            "{state} after {} configurations, {} events and {} work",
            self.configuration, self.event, self.work
        )?;
        if self.stop.is_empty() {
            return Ok(());
        }
        write!(formatter, ": {}", reason(&self.stop))
    }
}

impl From<&Laser> for Extent {
    fn from(laser: &Laser) -> Self {
        let summary = laser.summary();
        Self {
            closed: summary.closed,
            configuration: summary.state,
            event: summary.event,
            work: summary.work,
            stop: laser.stop(),
        }
    }
}

// What stopped an exploration, and the attributes that let it go on.
fn reason(stop: &[Stop]) -> String {
    let said = stop
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", and ");
    let mut raise = Vec::new();
    for name in stop.iter().filter_map(|stop| stop.raise()) {
        if !raise.contains(&name) {
            raise.push(name);
        }
    }
    if raise.is_empty() {
        return said;
    }
    format!("{said}; raise {}", raise.join(" and "))
}

// Where a failing test's details go, and the command that asks its question again.
struct Trail {
    directory: Option<PathBuf>,
    program: PathBuf,
}

impl Trail {
    fn record(&self, name: &str, report: &impl Serialize) -> miette::Result<()> {
        let Some(directory) = &self.directory else {
            return Ok(());
        };
        std::fs::write(
            directory.join(name),
            serde_json::to_vec_pretty(report).into_diagnostic()?,
        )
        .into_diagnostic()
    }

    // The photonic command that explores the tested program the same way: the assembled program
    // holds the sources and libraries, and the case adds its literal source and budgets.
    fn command(&self, case: &Case, verb: &str, question: &[String]) -> String {
        let source = ["--source".to_owned(), quote(&case.source)];
        let budget = [
            ("--work", case.work),
            ("--configuration", case.limit.configuration),
            ("--coherence", case.limit.coherence),
            ("--occurrence", case.limit.occurrence),
            ("--scope", case.limit.scope),
            ("--record", case.limit.record),
        ]
        .map(|(flag, value)| format!("{flag} {value}"));
        [
            "photonic".to_owned(),
            verb.to_owned(),
            quote(&self.program.display().to_string()),
        ]
        .into_iter()
        .chain(source.into_iter().filter(|_| !case.source.is_empty()))
        .chain(question.iter().cloned())
        .chain(budget)
        .collect::<Vec<_>>()
        .join(" ")
    }
}

fn quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

// A configuration as Photonic text: the root's coherences, then each scope's, as Spectrum prints
// them.
fn configuration(definition: &[Definition], node: &Node) -> String {
    let value = |value: &Value| match value {
        Value::Atom(atom) => source::Value::Atom(atom.to_string()),
        Value::Rule(index) => source::Value::Rule {
            rule: Box::new(definition[*index].rule.clone()),
        },
    };
    let mut frame = node
        .world
        .iter()
        .map(|world| world.frame)
        .collect::<Vec<_>>();
    frame.sort_unstable();
    frame.dedup();
    let part = frame
        .into_iter()
        .map(|index| {
            let text = node
                .world
                .iter()
                .filter(|world| world.frame == index)
                .map(|world| {
                    frontend::text::coherence(
                        &world
                            .particle
                            .iter()
                            .map(|token| value(&token.value))
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            if index == 0 {
                return text;
            }
            format!("in f{index}: {text}")
        })
        .collect::<Vec<_>>();
    if part.is_empty() {
        return "nothing".to_owned();
    }
    part.join(" · ")
}

fn show(label: &str, text: &[String]) {
    for text in text.iter().take(SHOWN) {
        println!("{label}: {text}");
    }
    if text.len() > SHOWN {
        println!(
            "{label}: {} more in the test's undeclared outputs",
            text.len() - SHOWN
        );
    }
}

// Every schedule of plain events ends exactly at a target when the plain exploration closes, no run
// can go on forever, and every configuration a run ends at is one of the targets. The reduced
// exploration keeps exactly those end configurations and cycles.
fn every(
    case: &Case,
    program: &Program,
    target: &[Program],
    trail: &Trail,
) -> miette::Result<bool> {
    let mut laser = Laser::reduced(program);
    laser.run(case.work, case.limit);
    let extent = Extent::from(&laser);
    let (verb, question) = match case.target.as_slice() {
        [single] => (
            "check",
            vec![
                "--plain".to_owned(),
                "--end".to_owned(),
                quote(single),
                "--exact".to_owned(),
                "--preserve".to_owned(),
            ],
        ),
        _ => ("explore", vec!["--plain".to_owned()]),
    };
    let command = trail.command(case, verb, &question);
    if !extent.closed {
        println!("Every schedule: {extent}");
        println!("Reproduce with: {command}");
        trail.record(
            "every.json",
            &serde_json::json!({"extent": extent, "command": command}),
        )?;
        return Ok(false);
    }
    let ending = laser.ending();
    let wanted = target
        .iter()
        .filter_map(|target| laser.verdict(target).witness)
        .collect::<Vec<_>>();
    let stray = ending
        .end
        .iter()
        .copied()
        .filter(|end| !wanted.contains(end))
        .collect::<Vec<_>>();
    println!(
        "Every schedule: {extent}; {} end configurations, {} of them not a target{}",
        ending.end.len(),
        stray.len(),
        if ending.endless {
            "; a run can go on forever"
        } else {
            ""
        },
    );
    if !ending.endless && stray.is_empty() {
        return Ok(true);
    }
    let report = laser.report();
    let text = |index: usize| configuration(&report.definition, &report.state[index]);
    let end = stray.iter().map(|&index| text(index)).collect::<Vec<_>>();
    show("End", &end);
    let mut outgoing = vec![Vec::new(); report.state.len()];
    for event in report
        .event
        .iter()
        .filter(|event| event.status == Status::Supported)
    {
        outgoing[event.source].push(event.target);
    }
    let cycle = ending
        .endless
        .then(|| {
            photonic::laser::ending::cycle(report.state.len(), |node| {
                outgoing[node].iter().map(|&target| ((), target))
            })
        })
        .flatten()
        .map(|(node, _)| text(node));
    if let Some(cycle) = &cycle {
        println!("Forever: a run returns to {cycle}");
    }
    println!("Reproduce with: {command}");
    trail.record(
        "every.json",
        &serde_json::json!({"extent": extent, "end": end, "cycle": cycle, "command": command}),
    )?;
    Ok(false)
}

// Each target's outcome against the one expected: the interpreter's, which Laser must agree with
// wherever both settle, or one direct path's. What a failure found goes to the test's undeclared
// outputs, so a failing test can be read without running it again.
fn prism(
    case: &Case,
    program: &Program,
    target: Vec<Program>,
    expected: Outcome,
    trail: &Trail,
) -> miette::Result<bool> {
    let exploration = (!case.path).then(|| {
        let mut runtime = Runtime::new(program);
        runtime.run(case.work, case.limit);
        let mut laser = Laser::new(program);
        laser.run(case.work, case.limit);
        (runtime, laser)
    });
    let mut success = true;
    for (index, target) in target.into_iter().enumerate() {
        let name = format!("{index}.json");
        let text = &case.target[index];
        if let Some((runtime, laser)) = &exploration {
            let verdict = runtime.verdict(&target);
            let compiled = laser.verdict(&target).outcome;
            let agree = compiled == verdict.outcome
                || !runtime.closed()
                    && (compiled == Outcome::Unknown || verdict.outcome == Outcome::Unknown);
            println!("Prism target {index}: {:?}; {text}", verdict.outcome);
            if verdict.outcome == expected && agree {
                continue;
            }
            success = false;
            let snapshot = runtime.snapshot();
            let interpreted = Extent {
                closed: snapshot.closed,
                configuration: snapshot.state.len(),
                event: snapshot.event.len(),
                work: snapshot.work,
                stop: snapshot.stop,
            };
            let explored = Extent::from(laser);
            println!("Interpreter: {:?}, {interpreted}", verdict.outcome);
            println!("Laser: {compiled:?}, {explored}");
            if !agree {
                println!("The engines disagree where both settle");
            }
            let claim = match expected {
                Outcome::Unreachable => "--avoid",
                _ => "--reach",
            };
            let question = [claim, &quote(text), "--exact", "--preserve"].map(str::to_owned);
            let command = trail.command(case, "check", &question);
            println!("Reproduce with: {command}");
            trail.record(
                &name,
                &serde_json::json!({
                    "target": text,
                    "interpreter": {"outcome": verdict.outcome, "witness": verdict.witness, "extent": interpreted},
                    "laser": {"outcome": compiled, "extent": explored},
                    "command": command,
                }),
            )?;
            continue;
        }
        let mut search = photonic::path::Search::new(program.clone(), Some(target));
        search.run(case.work, case.limit);
        let summary = search.summary();
        println!("Prism target {index}: {:?}; {text}", summary.outcome);
        if summary.outcome == expected {
            continue;
        }
        success = false;
        let report = search.report();
        println!(
            "Path: {} events and {} work, ending at {}",
            summary.length,
            summary.work,
            configuration(&report.definition, &search.current())
        );
        println!("Path stopped: {}", reason(&summary.stop));
        let question = ["--path", "--goal", &quote(text), "--preserve"].map(str::to_owned);
        let command = trail.command(case, "check", &question);
        println!("Reproduce with: {command}");
        trail.record(&name, &report)?;
    }
    if let (false, Some((runtime, _))) = (success, &exploration) {
        trail.record("execution.json", &runtime.stream())?;
    }
    Ok(success)
}

fn finish(prefix: &str, success: bool) -> ExitCode {
    if !success {
        println!("{prefix}failed");
        return ExitCode::FAILURE;
    }
    println!("{prefix}passed");
    ExitCode::SUCCESS
}

fn lower(source: &str) -> miette::Result<Program> {
    frontend::lowering::parse(source)
        .map_err(|failure| miette::Report::new(failure).with_source_code(source.to_string()))
}

fn main() -> miette::Result<ExitCode> {
    let runfile = runfiles::Runfiles::create().into_diagnostic()?;
    let resolve = |name: &str| {
        runfile
            .rlocation_from(name, "")
            .ok_or_else(|| miette::miette!("missing runfile: {name}"))
    };
    let argument = std::env::args().skip(1).collect::<Vec<_>>();
    let [configuration] = argument.as_slice() else {
        miette::bail!("expected one test configuration runfile");
    };
    let case: Case =
        serde_json::from_slice(&std::fs::read(resolve(configuration)?).into_diagnostic()?)
            .into_diagnostic()
            .wrap_err("invalid test configuration")?;
    if case.target.is_empty() {
        miette::bail!("a test needs at least one target configuration");
    }
    let path = resolve(&case.program)?;
    let mut program = Program::read(&std::fs::read_to_string(&path).into_diagnostic()?)
        .into_diagnostic()
        .wrap_err("invalid assembled program")?;
    program.append(lower(&case.source)?);
    let target = case
        .target
        .iter()
        .map(|source| {
            let mut target = lower(source)?;
            target.preserve(&program);
            Ok(target)
        })
        .collect::<miette::Result<Vec<_>>>()?;
    let trail = Trail {
        directory: std::env::var_os("TEST_UNDECLARED_OUTPUTS_DIR").map(PathBuf::from),
        program: std::fs::canonicalize(&path).unwrap_or(path),
    };
    if case.every {
        if case.path || matches!(case.expect, Expect::Unreachable) {
            miette::bail!(
                "every schedule ends at a target explores every plain schedule and expects reached"
            );
        }
        return Ok(finish(
            "Every schedule ends at a target: ",
            every(&case, &program, &target, &trail)?,
        ));
    }
    let expected = match (case.expect, case.path) {
        (Expect::Reached, _) => Outcome::Reached,
        (Expect::Unreachable, false) => Outcome::Unreachable,
        (Expect::Unreachable, true) => {
            miette::bail!("a direct path can witness reachability but cannot prove unreachability")
        }
    };
    let success = prism(&case, &program, target, expected, &trail)?;
    Ok(finish(&format!("Prism: expected {expected:?}; "), success))
}
