use clap::Parser;
use std::num::NonZeroUsize;

mod evaluation;
mod formula;
mod limit;
mod warm;

#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Parser)]
struct Argument {
    input: String,
    #[arg(allow_hyphen_values = true)]
    expected: String,
    #[arg(long, default_value = "5")]
    sample: NonZeroUsize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let (program, target) =
        formula::program(&formula::source(&argument.input, &argument.expected)?)?;
    evaluation::warm(&program, &target, None)?;
    let measurement = (0..argument.sample.get())
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
