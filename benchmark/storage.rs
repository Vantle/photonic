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
    #[arg(long)]
    rule: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let mut context = (0..argument.width)
        .map(|index| format!("Idle{index},"))
        .collect::<String>();
    let mut source = format!("{context}Stage0,\n");
    if argument.rule {
        for index in 0..argument.width {
            source.push_str(&format!("[Idle{index},Idle{index}] Never{index},\n"));
        }
    }
    for index in 0..argument.length {
        source.push_str(&format!("[Stage{index}] Stage{},\n", index + 1));
    }
    context.push_str(&format!("Stage{}", argument.length));
    let program = frontend::lowering::parse(&source)?;
    let mut target = frontend::lowering::parse(&context)?;
    target.preserve(&program);
    let limit = Limit {
        configuration: argument.length + 1,
        record: 100_000_000,
        coherence: argument.width + 1,
        occurrence: argument.width + 1,
        scope: 1,
    };
    evaluation::warm(&program, &target, Some(limit))?;
    let measurement = (0..argument.sample)
        .map(|_| evaluation::evaluate(program.clone(), target.clone(), Some(limit)))
        .collect::<Result<Vec<_>, _>>()?;
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width, "length": argument.length, "rule": argument.rule,
            "measurement": measurement,
        }),
    )?;
    Ok(())
}
