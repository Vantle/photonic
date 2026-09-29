use crate::argument;
use crate::verb;
use miette::IntoDiagnostic;
use photonic::laser::net::{Cycle, Exploration, Net};
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

// The backend an exploration ran on.
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum Backend {
    Metal,
    Host,
}

#[derive(Serialize)]
struct Answer {
    engine: Backend,
    closed: bool,
    configuration: usize,
    event: u64,
    endless: Option<bool>,
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
fn explore(net: &mut Net, limit: Limit, host: bool) -> miette::Result<(Exploration, Backend)> {
    let device = if host {
        None
    } else {
        wave::engine::Engine::new().into_diagnostic()?
    };
    if let Some(device) = device {
        let explored = device.explore(net, limit, Cycle::Find).into_diagnostic()?;
        return Ok((explored, Backend::Metal));
    }
    Ok((
        net.explore(limit, Cycle::Find).into_diagnostic()?,
        Backend::Host,
    ))
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
    let mut net = Net::new(&program).into_diagnostic()?;
    let (explored, engine) = explore(&mut net, limit, argument.host)?;
    // A cycle among the configurations found is a run that goes on forever, but an open exploration
    // without one leaves it unknown, since what the limits refused could close a cycle.
    let endless = match explored.endless {
        Some(true) => Some(true),
        _ if explored.closed => Some(false),
        _ => None,
    };
    let found = target
        .iter()
        .map(|value| net.find(value))
        .collect::<Vec<_>>();
    let checked = !target.is_empty();
    let definition = net.definition();
    let end = explored
        .end
        .iter()
        .take(argument.list)
        .map(|marking| End {
            text: crate::display(&net.node(marking), &definition),
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
    let passed = stray.map(|stray| explored.closed && endless == Some(false) && stray == 0);
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
        match engine {
            Backend::Metal => "Metal",
            Backend::Host => "the host",
        },
        count(explored.end.len() as u64, "end configuration"),
        answer
            .stray
            .map(|stray| format!(", {stray} of them not a target"))
            .unwrap_or_default(),
        match endless {
            Some(true) => "a run can go on forever",
            Some(false) => "no run goes on forever",
            None => "whether a run goes on forever is unknown",
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
