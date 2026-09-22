use clap::Parser;

mod evaluation;
mod formula;

#[derive(Parser)]
struct Argument {
    input: String,
    expected: String,
    #[arg(long, default_value_t = 5)]
    sample: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    if argument.input.is_empty()
        || !argument
            .input
            .chars()
            .all(|value| "012+-*/()".contains(value))
    {
        return Err("use a nonempty ternary expression".into());
    }
    if argument.expected.is_empty() || !argument.expected.chars().all(|value| "012".contains(value))
    {
        return Err("use a nonnegative ternary expected result".into());
    }
    let source = formula::source(&argument.input, &argument.expected);
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
            "input": argument.input, "expected": argument.expected, "measurement": measurement,
        }),
    )?;
    Ok(())
}
