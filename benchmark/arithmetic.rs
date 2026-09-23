use clap::{Parser, ValueEnum};
mod evaluation;
mod formula;

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
    let source = formula::source(&input, &ternary)?;
    let mut program: photonic::source::Program =
        serde_json::from_str(include_str!(env!("FORMULA")))?;
    let encoded = photonic::lowering::parse(&source)?;
    program.initial = encoded.initial;
    program.rule.extend(encoded.rule);
    let target = photonic::lowering::parse("Done.Zero")?;
    let target = photonic::source::Program {
        rule: program.rule.clone(),
        ..target
    };
    evaluation::warm(&program, &target, None);
    let measurement = (0..argument.sample)
        .map(|_| evaluation::evaluate(program.clone(), target.clone(), None))
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
