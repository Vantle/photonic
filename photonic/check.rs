use frontend::source::Program;
use miette::{IntoDiagnostic, WrapErr};
use photonic::laser::Laser;
use photonic::prism::Outcome;
use photonic::runtime::{Limit, Runtime};
use photonic::stop::Stop;
use serde::Deserialize;
use std::process::ExitCode;

#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

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

// Why an exploration stopped short of settling the test, and the attributes that let it go on.
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

// Every schedule of plain events ends exactly at a target when the plain exploration closes, no run
// can go on forever, and every configuration a run ends at is one of the targets. The reduced
// exploration keeps exactly those end configurations and cycles.
fn every(program: &Program, target: &[Program], limit: Limit) -> bool {
    let mut laser = Laser::reduced(program);
    laser.run(usize::MAX, limit);
    let summary = laser.summary();
    if !summary.closed {
        println!(
            "Every schedule: open after {} configurations and {} events: {}",
            summary.state,
            summary.event,
            reason(&laser.stop())
        );
        return false;
    }
    let ending = laser.ending();
    let wanted = target
        .iter()
        .filter_map(|target| laser.verdict(target).witness)
        .collect::<Vec<_>>();
    let stray = ending
        .end
        .iter()
        .filter(|end| !wanted.contains(end))
        .count();
    println!(
        "Every schedule: closed after {} configurations and {} events; {} end configurations, {stray} of them not a target{}",
        summary.state,
        summary.event,
        ending.end.len(),
        if ending.endless {
            "; a run can go on forever"
        } else {
            ""
        },
    );
    !ending.endless && stray == 0
}

// Each target's outcome against the one expected: the interpreter's, which Laser must agree with
// wherever both settle, or one direct path's. What a failure found goes to the test's undeclared
// outputs, so a failing test can be read without running it again.
fn prism(
    case: &Case,
    program: &Program,
    target: Vec<Program>,
    expected: Outcome,
) -> miette::Result<bool> {
    let exploration = (!case.path).then(|| {
        let mut runtime = Runtime::new(program);
        runtime.run(case.work, case.limit);
        let mut laser = Laser::new(program);
        laser.run(usize::MAX, case.limit);
        (runtime, laser)
    });
    let directory = std::env::var_os("TEST_UNDECLARED_OUTPUTS_DIR").map(std::path::PathBuf::from);
    let mut success = true;
    for (index, target) in target.into_iter().enumerate() {
        let name = format!("{index}.json");
        let result = if let Some((runtime, laser)) = &exploration {
            let verdict = runtime.verdict(&target);
            let compiled = laser.verdict(&target).outcome;
            let agree = compiled == verdict.outcome
                || !runtime.closed()
                    && (compiled == Outcome::Unknown || verdict.outcome == Outcome::Unknown);
            if !agree {
                println!(
                    "Laser answers {compiled:?} where the interpreter answers {:?}",
                    verdict.outcome
                );
            }
            success &= agree;
            if verdict.outcome != expected || !agree {
                record(
                    &directory,
                    &name,
                    &serde_json::json!({
                        "target": case.target[index],
                        "outcome": verdict.outcome,
                        "witness": verdict.witness,
                        "laser": compiled,
                    }),
                )?;
            }
            verdict.outcome
        } else {
            let mut search = photonic::path::Search::new(program.clone(), Some(target));
            search.run(case.work, case.limit);
            let summary = search.summary();
            if summary.outcome != expected {
                record(&directory, &name, &search.report())?;
                println!("Path stopped: {}", reason(&summary.stop));
            }
            summary.outcome
        };
        println!("Prism target {index}: {result:?}; {}", case.target[index]);
        success &= result == expected;
    }
    if let (false, Some((runtime, laser))) = (success, &exploration) {
        for (engine, stop) in [("Interpreter", runtime.stop()), ("Laser", laser.stop())] {
            if !stop.is_empty() {
                println!("{engine} stopped: {}", reason(&stop));
            }
        }
        record(&directory, "execution.json", &runtime.stream())?;
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
    let mut program =
        Program::read(&std::fs::read_to_string(resolve(&case.program)?).into_diagnostic()?)
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
    if case.every {
        if case.path || matches!(case.expect, Expect::Unreachable) {
            miette::bail!(
                "every schedule ends at a target explores every plain schedule and expects reached"
            );
        }
        return Ok(finish(
            "Every schedule ends at a target: ",
            every(&program, &target, case.limit),
        ));
    }
    let expected = match (case.expect, case.path) {
        (Expect::Reached, _) => Outcome::Reached,
        (Expect::Unreachable, false) => Outcome::Unreachable,
        (Expect::Unreachable, true) => {
            miette::bail!("a direct path can witness reachability but cannot prove unreachability")
        }
    };
    let success = prism(&case, &program, target, expected)?;
    Ok(finish(&format!("Prism: expected {expected:?}; "), success))
}

fn record(
    directory: &Option<std::path::PathBuf>,
    name: &str,
    report: &impl serde::Serialize,
) -> miette::Result<()> {
    let Some(directory) = directory else {
        return Ok(());
    };
    std::fs::write(
        directory.join(name),
        serde_json::to_vec_pretty(report).into_diagnostic()?,
    )
    .into_diagnostic()
}
