use molten::lowering::parse;
use molten::obsidian::{Outcome, Search};
use molten::snapshot::Snapshot;
use molten::source::{Program, Value};
use serde::Serialize;
use std::io::{self, Write};

#[derive(Serialize)]
struct Case {
    title: String,
    source: String,
    description: String,
    outcome: Outcome,
    witness: Option<usize>,
    program: Program,
    target: Vec<Vec<Value>>,
    report: Snapshot,
}

fn record(
    title: String,
    initial: String,
    step: usize,
    expected: Outcome,
) -> Result<Case, Box<dyn std::error::Error>> {
    let template = parse(include_str!("membership.lava"))?;
    let declaration = template
        .rule
        .iter()
        .map(|rule| rule.name.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let source = format!("{initial}\n{declaration}\n");
    let program = parse(&source)?;
    let mut search = Search::new(program, parse(include_str!("natural.lava"))?)?;
    search.run(step, None);
    let result = search.report();
    assert_eq!(result.outcome, expected, "{title}");
    let description = match result.outcome {
        Outcome::Reached => format!(
            "Obsidian: reached exact Natural at configuration {}.",
            result.witness.unwrap()
        ),
        Outcome::Unreachable => {
            "Obsidian: exact Natural is unreachable in this closed graph.".into()
        }
        Outcome::Unknown => {
            "Obsidian: unknown with a zero-step budget. No application has been explored.".into()
        }
    };
    Ok(Case {
        title,
        source,
        description,
        outcome: result.outcome,
        witness: result.witness,
        program: result.program,
        target: result.target,
        report: result.execution,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut evidence = Vec::new();
    for count in 0..=6 {
        evidence.push(record(
            format!("Numeral {count}"),
            format!("Check{}", ".Unit".repeat(count)),
            12_000,
            Outcome::Reached,
        )?);
    }
    for (title, initial) in [
        ("Unknown atom", "Check.Unit.Other"),
        ("Extra Natural marker", "Check.Unit.Natural"),
        ("Extra Check marker", "Check.Unit.Check"),
        ("Zero is not a built-in label", "Check.Zero"),
    ] {
        evidence.push(record(
            title.into(),
            initial.into(),
            12_000,
            Outcome::Unreachable,
        )?);
    }
    evidence.push(record(
        "Paused before exploration".into(),
        "Check.Unit.Unit".into(),
        0,
        Outcome::Unknown,
    )?);
    let mut output = io::stdout().lock();
    write!(output, "globalThis.native = ")?;
    serde_json::to_writer(&mut output, &evidence)?;
    writeln!(output, ";")?;
    Ok(())
}
