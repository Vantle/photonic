use crate::argument;
use crate::verb;
use miette::IntoDiagnostic;
use photonic::laser::ground::{Exploration, Ground};
use photonic::runtime::Limit;
use serde::Serialize;
use std::io::Write;
use std::process::ExitCode;

#[derive(Serialize)]
struct End {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<bool>,
}

#[derive(Serialize)]
struct Answer {
    engine: &'static str,
    closed: bool,
    configuration: usize,
    event: u64,
    endless: bool,
    end: Vec<End>,
    more: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    stray: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    passed: Option<bool>,
}

fn count(value: u64, noun: &str) -> String {
    if value == 1 {
        return format!("1 {noun}");
    }
    format!("{value} {noun}s")
}

// The GPU where there is one and the caller did not ask for the host, and the host's net otherwise;
// both explore alike, number for number.
fn explore(
    ground: &mut Ground,
    limit: Limit,
    host: bool,
) -> miette::Result<(Exploration, &'static str)> {
    if let Some(engine) = (!host).then(wave::engine::Engine::new).and_then(Result::ok) {
        return Ok((
            engine.explore(ground, limit, true).into_diagnostic()?,
            "metal",
        ));
    }
    Ok((ground.explore(limit).into_diagnostic()?, "host"))
}

pub fn every(argument: &argument::Every) -> miette::Result<ExitCode> {
    let program = match crate::load(&argument.file, &argument.source) {
        Ok(program) => program,
        Err(failure) => return verb::fail(&failure),
    };
    let mut target = Vec::with_capacity(argument.target.len());
    for path in &argument.target {
        match crate::load(std::slice::from_ref(path), &argument::Source::default()) {
            Ok(mut value) => {
                if argument.preserve {
                    value.preserve(&program);
                }
                target.push(value);
            }
            Err(failure) => return verb::fail(&failure),
        }
    }
    let limit = Limit {
        configuration: argument.configuration,
        coherence: argument.coherence,
        occurrence: argument.occurrence,
        scope: argument.scope,
        record: usize::MAX,
    };
    let mut ground = Ground::new(&program).into_diagnostic()?;
    let (explored, engine) = explore(&mut ground, limit, argument.host)?;
    let endless = explored.endless.unwrap_or_default();
    let found = target
        .iter()
        .map(|value| ground.find(value))
        .collect::<Vec<_>>();
    let checked = !target.is_empty();
    let definition = ground.definition();
    let end = explored
        .end
        .iter()
        .take(argument.list)
        .map(|marking| End {
            text: crate::display(&ground.node(marking), &definition),
            target: checked.then(|| found.contains(&Some(marking.clone()))),
        })
        .collect::<Vec<_>>();
    let stray = checked.then(|| {
        explored
            .end
            .iter()
            .filter(|marking| !found.contains(&Some((*marking).clone())))
            .count()
    });
    let passed = stray.map(|stray| explored.closed && !endless && stray == 0);
    let answer = Answer {
        engine,
        closed: explored.closed,
        configuration: explored.configuration,
        event: explored.event,
        endless,
        more: explored.end.len().saturating_sub(argument.list),
        end,
        stray,
        passed,
    };
    let code = if explored.closed && passed != Some(false) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    };
    if argument.json {
        crate::output::write(&answer, false)?;
        return Ok(code);
    }
    let mut output = std::io::stdout().lock();
    writeln!(
        output,
        "Every schedule: {} after {} and {} on {}; {}{}; {}",
        if answer.closed { "closed" } else { "open" },
        count(answer.configuration as u64, "configuration"),
        count(answer.event, "event"),
        if engine == "metal" {
            "Metal"
        } else {
            "the host"
        },
        count(explored.end.len() as u64, "end configuration"),
        answer
            .stray
            .map(|stray| format!(", {stray} of them not a target"))
            .unwrap_or_default(),
        if endless {
            "a run can go on forever"
        } else {
            "no run goes on forever"
        },
    )
    .into_diagnostic()?;
    for (index, value) in answer.end.iter().enumerate() {
        let mark = match value.target {
            Some(true) => " target",
            Some(false) => " stray",
            None => "",
        };
        writeln!(output, "end {}{mark}: {}", index + 1, value.text).into_diagnostic()?;
    }
    if answer.more > 0 {
        writeln!(
            output,
            "{} more",
            count(answer.more as u64, "end configuration")
        )
        .into_diagnostic()?;
    }
    if let Some(passed) = answer.passed {
        writeln!(
            output,
            "Every schedule ends at a target: {}",
            if passed { "passed" } else { "failed" }
        )
        .into_diagnostic()?;
    }
    Ok(code)
}
