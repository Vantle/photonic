use clap::Parser;

mod evaluation;
mod formula;

#[derive(Parser)]
struct Argument {
    input: String,
    #[arg(allow_hyphen_values = true)]
    expected: String,
    #[arg(long, default_value_t = 5)]
    sample: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let (program, target) =
        formula::program(&formula::source(&argument.input, &argument.expected)?)?;
    evaluation::warm(&program, &target, None)?;
    let measurement = (0..argument.sample)
        .map(|_| evaluation::evaluate(program.clone(), target.clone(), None))
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "input": argument.input, "expected": argument.expected, "measurement": measurement,
        }),
    )?;
    Ok(())
}
