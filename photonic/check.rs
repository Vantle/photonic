use photonic::prism::{Outcome, Search};
use photonic::runtime::Limit;
use photonic::source::Program;
use serde::Deserialize;
use std::process::ExitCode;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    program: String,
    source: String,
    targets: Vec<String>,
    expect: String,
    #[serde(rename = "match")]
    mode: String,
    path: bool,
    preserve: bool,
    step: usize,
    state: usize,
    cell: usize,
    frame: usize,
    coherence: usize,
    record: usize,
}

fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let runfile = runfiles::Runfiles::create()?;
    let resolve = |name: &str| {
        runfile
            .rlocation_from(name, "")
            .ok_or_else(|| format!("missing runfile: {name}"))
    };
    let argument = std::env::args().skip(1).collect::<Vec<_>>();
    if argument.len() != 1 {
        return Err("expected one test configuration runfile".into());
    }
    let case: Case = serde_json::from_slice(&std::fs::read(resolve(&argument[0])?)?)?;
    if case.targets.is_empty() {
        return Err("a test needs at least one target configuration".into());
    }
    let mut program: Program = serde_json::from_slice(&std::fs::read(resolve(&case.program)?)?)?;
    let source = photonic::lowering::parse(&case.source)?;
    program.initial.extend(source.initial);
    program.rule.extend(source.rule);
    let target = case
        .targets
        .iter()
        .map(|source| {
            let mut target = photonic::lowering::parse(source)?;
            if case.preserve {
                target.rule.extend(program.rule.iter().cloned());
            }
            Ok(target)
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let expected = match case.expect.as_str() {
        "reached" => Outcome::Reached,
        "unreachable" if !case.path => Outcome::Unreachable,
        _ => return Err("expected reached, or unreachable with full exploration".into()),
    };
    let every = match case.mode.as_str() {
        "all" => true,
        "any" => false,
        _ => return Err("match must be all or any".into()),
    };
    let limit = Limit {
        state: case.state,
        cell: case.cell,
        frame: case.frame,
        world: case.coherence,
        record: case.record,
    };
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
            serde_json::to_vec_pretty(&search.report())?,
        )?;
    }
    let mut success = every;
    for (index, target) in target.into_iter().enumerate() {
        let (result, report) = if let Some(search) = &mut exploration {
            search.target(target);
            let verdict = search.verdict();
            (
                verdict.outcome,
                serde_json::to_vec_pretty(&serde_json::json!({
                    "target": case.targets[index],
                    "outcome": verdict.outcome,
                    "witness": verdict.witness,
                }))?,
            )
        } else {
            let mut search = photonic::path::Search::new(program.clone(), target);
            search.run(case.step, limit);
            let report = search.report();
            (report.outcome, serde_json::to_vec_pretty(&report)?)
        };
        if let Some(directory) = &directory {
            std::fs::write(
                std::path::Path::new(&directory).join(format!("{index}.json")),
                &report,
            )?;
        }
        println!("Prism target {index}: {result:?}; {}", case.targets[index]);
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
        case.mode,
        if success { "passed" } else { "failed" }
    );
    Ok(if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
