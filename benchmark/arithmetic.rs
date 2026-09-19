use clap::{Parser, ValueEnum};
use photonic::path::Search;
use photonic::prism::Outcome;
use photonic::runtime::Limit;
use serde::Serialize;
use std::time::Instant;

#[derive(Clone, Copy, ValueEnum)]
enum Operation {
    Add,
    Multiply,
}

#[derive(Parser)]
struct Argument {
    left: String,
    operation: Operation,
    right: String,
    #[arg(long, default_value_t = 3)]
    radix: u32,
    #[arg(long, default_value_t = 3)]
    sample: usize,
}

#[derive(Serialize)]
struct Measurement {
    initialization: f64,
    execution: f64,
    event: usize,
    work: usize,
}

fn numeral(mut value: u128) -> String {
    let mut digit = Vec::new();
    loop {
        digit.push(char::from(b'0' + (value % 3) as u8));
        value /= 3;
        if value == 0 {
            return digit.into_iter().rev().collect();
        }
    }
}

fn source(input: &str, expected: &str) -> String {
    let token = input
        .chars()
        .rev()
        .map(|value| match value {
            '+' => "Add".to_owned(),
            '*' => "Multiply".to_owned(),
            digit => format!("([Digit] {digit})"),
        })
        .collect::<Vec<_>>();
    let mut source = format!("Push.{}.Zero, Stage.1\n", token[0]);
    for (index, value) in token.iter().enumerate().skip(1) {
        let next = index + 1;
        source.push_str(&format!("[Built,Stage.{index}] (Push.{value}) (Forget.Stage.{next})\n[Clean.Stage.{next}] Stage.{next}\n"));
    }
    source.push_str(&format!("[Built,Stage.{}] (Function.Expression) (Forget.Inspect.0)\n[Clean.Inspect.0] Inspect.0\n[Return.Expression.Number.Positive,Inspect.0] (Read) (Forget.Inspect.1)\n[Clean.Inspect.1] Inspect.1\n", token.len()));
    for (index, digit) in expected.chars().rev().enumerate() {
        let current = index + 1;
        let next = current + 1;
        if current == expected.len() {
            source.push_str(&format!(
                "[Yield.([Digit] {digit}).Zero,Inspect.{current}] Done.Zero\n"
            ));
        } else {
            source.push_str(&format!("[Yield.([Digit] {digit}),Inspect.{current}] (Read) (Forget.Inspect.{next})\n[Clean.Inspect.{next}] Inspect.{next}\n"));
        }
    }
    source
}

fn evaluate(program: photonic::source::Program) -> Measurement {
    let start = Instant::now();
    let mut search = Search::new(program, photonic::lowering::parse("Done.Zero").unwrap()).unwrap();
    let initialization = start.elapsed().as_secs_f64();
    let start = Instant::now();
    search.run(
        100_000_000,
        Limit {
            state: 262_144,
            record: 100_000_000,
            world: 1024,
            cell: 16_384,
            frame: 2048,
        },
    );
    let execution = start.elapsed().as_secs_f64();
    let summary = search.summary();
    assert_eq!(summary.outcome, Outcome::Reached);
    Measurement {
        initialization,
        execution,
        event: summary.event,
        work: summary.work,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    if !(2..=36).contains(&argument.radix) {
        return Err("radix must be between 2 and 36".into());
    }
    let left = u128::from_str_radix(&argument.left, argument.radix)?;
    let right = u128::from_str_radix(&argument.right, argument.radix)?;
    let (operator, expected) = match argument.operation {
        Operation::Add => ('+', left.checked_add(right)),
        Operation::Multiply => ('*', left.checked_mul(right)),
    };
    let expected = expected.ok_or("reference arithmetic overflow")?;
    if left == 0 || right == 0 {
        return Err("use positive operands for this benchmark".into());
    }
    let input = format!("{}{operator}{}", numeral(left), numeral(right));
    let ternary = numeral(expected);
    let source = source(&input, &ternary);
    let mut program: photonic::source::Program =
        serde_json::from_str(include_str!(env!("FORMULA")))?;
    let encoded = photonic::lowering::parse(&source)?;
    program.initial = encoded.initial;
    program.rule.extend(encoded.rule);
    evaluate(program.clone());
    let measurement = (0..argument.sample)
        .map(|_| evaluate(program.clone()))
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "input": input, "ternary": ternary, "decimal": format!("{left} {operator} {right} = {expected}"),
            "source": source, "measurement": measurement,
        }),
    )?;
    Ok(())
}
