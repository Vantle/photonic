use frontend::source::Program;
use miette::{IntoDiagnostic, WrapErr};
use photonic::prism::Outcome;
use photonic::runtime::{Limit, Runtime};
use serde::Deserialize;
use std::process::ExitCode;

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
    work: usize,
    limit: Limit,
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
    let expected = match (case.expect, case.path) {
        (Expect::Reached, _) => Outcome::Reached,
        (Expect::Unreachable, false) => Outcome::Unreachable,
        (Expect::Unreachable, true) => {
            miette::bail!("a direct path can witness reachability but cannot prove unreachability")
        }
    };
    let limit = case.limit;
    let exploration = (!case.path).then(|| {
        let mut runtime = Runtime::new(&program);
        runtime.run(case.work, limit);
        runtime
    });
    let directory = std::env::var_os("TEST_UNDECLARED_OUTPUTS_DIR").map(std::path::PathBuf::from);
    let mut success = true;
    for (index, target) in target.into_iter().enumerate() {
        let name = format!("{index}.json");
        let result = if let Some(runtime) = &exploration {
            let verdict = runtime.verdict(&target);
            if verdict.outcome != expected {
                record(
                    &directory,
                    &name,
                    &serde_json::json!({
                        "target": case.target[index],
                        "outcome": verdict.outcome,
                        "witness": verdict.witness,
                    }),
                )?;
            }
            verdict.outcome
        } else {
            let mut search = photonic::path::Search::new(program.clone(), Some(target));
            search.run(case.work, limit);
            let outcome = search.summary().outcome;
            if outcome != expected {
                record(&directory, &name, &search.report())?;
            }
            outcome
        };
        println!("Prism target {index}: {result:?}; {}", case.target[index]);
        success &= result == expected;
    }
    println!(
        "Prism: expected {expected:?}; {}",
        if success { "passed" } else { "failed" }
    );
    if let (false, Some(runtime)) = (success, &exploration) {
        record(&directory, "execution.json", &runtime.stream())?;
    }
    Ok(if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
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
