use clap::Parser;
use photonic::runtime::Limit;

mod evaluation;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value_t = 1000)]
    width: usize,
    #[arg(long, default_value_t = 1000)]
    length: usize,
    #[arg(long, default_value_t = 5)]
    sample: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let mut context = (0..argument.width)
        .map(|index| format!("Idle{index},"))
        .collect::<String>();
    let mut source = format!("{context}Stage0\n");
    for index in 0..argument.length {
        source.push_str(&format!("[Stage{index}] Stage{}\n", index + 1));
    }
    context.push_str(&format!("Stage{}", argument.length));
    let program = photonic::lowering::parse(&source)?;
    let target = photonic::lowering::parse(&context)?;
    let limit = Limit {
        state: argument.length + 1,
        record: 100_000_000,
        world: argument.width + 1,
        cell: argument.width + 1,
        frame: 1,
    };
    evaluation::evaluate(program.clone(), target.clone(), Some(limit));
    let measurement = (0..argument.sample)
        .map(|_| evaluation::evaluate(program.clone(), target.clone(), Some(limit)))
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width, "length": argument.length, "measurement": measurement,
        }),
    )?;
    Ok(())
}
