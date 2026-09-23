use miette::{IntoDiagnostic, WrapErr};
use photonic::prism::{Outcome, Search};
use photonic::runtime::Limit;
use photonic::source::Program;
use serde::Deserialize;
use std::process::ExitCode;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Expect {
    Reached,
    Unreachable,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Mode {
    All,
    Any,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    program: String,
    source: String,
    target: Vec<String>,
    expect: Expect,
    #[serde(rename = "match")]
    mode: Mode,
    path: bool,
    preserve: bool,
    step: usize,
    limit: Limit,
}

fn lower(source: &str) -> miette::Result<Program> {
    photonic::lowering::parse(source)
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
    let mut program: Program =
        serde_json::from_slice(&std::fs::read(resolve(&case.program)?).into_diagnostic()?)
            .into_diagnostic()
            .wrap_err("invalid assembled program")?;
    program.append(lower(&case.source)?);
    let target = case
        .target
        .iter()
        .map(|source| {
            let mut target = lower(source)?;
            if case.preserve {
                target.rule.extend(program.rule.iter().cloned());
            }
            Ok(target)
        })
        .collect::<miette::Result<Vec<_>>>()?;
    let expected = match (case.expect, case.path) {
        (Expect::Reached, _) => Outcome::Reached,
        (Expect::Unreachable, false) => Outcome::Unreachable,
        (Expect::Unreachable, true) => {
            miette::bail!("a direct path can witness reachability but cannot prove unreachability")
        }
    };
    let every = matches!(case.mode, Mode::All);
    let limit = case.limit;
    let mut exploration = if case.path {
        None
    } else {
        let mut search = Search::new(program.clone(), target[0].clone());
        search.run(case.step, Some(limit));
        Some(search)
    };
    let directory = std::env::var_os("TEST_UNDECLARED_OUTPUTS_DIR").map(std::path::PathBuf::from);
    if let (Some(search), Some(directory)) = (&exploration, &directory) {
        std::fs::write(
            directory.join("execution.json"),
            serde_json::to_vec_pretty(&search.report()).into_diagnostic()?,
        )
        .into_diagnostic()?;
    }
    let mut success = every;
    for (index, target) in target.into_iter().enumerate() {
        let (result, report) = if let Some(search) = &mut exploration {
            search.target(target);
            let verdict = search.verdict();
            (
                verdict.outcome,
                serde_json::to_vec_pretty(&serde_json::json!({
                    "target": case.target[index],
                    "outcome": verdict.outcome,
                    "witness": verdict.witness,
                }))
                .into_diagnostic()?,
            )
        } else {
            let mut search = photonic::path::Search::new(program.clone(), target);
            search.run(case.step, limit);
            let report = search.report();
            (
                report.outcome,
                serde_json::to_vec_pretty(&report).into_diagnostic()?,
            )
        };
        if let Some(directory) = &directory {
            std::fs::write(
                std::path::Path::new(&directory).join(format!("{index}.json")),
                &report,
            )
            .into_diagnostic()?;
        }
        println!("Prism target {index}: {result:?}; {}", case.target[index]);
        let matched = result == expected;
        success = if every {
            success && matched
        } else {
            success || matched
        };
        if !every && success {
            break;
        }
    }
    println!(
        "Prism: match {}; expected {expected:?}; {}",
        if every { "all" } else { "any" },
        if success { "passed" } else { "failed" }
    );
    Ok(if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
