use clap::Parser;
use photonic::runtime::Limit;
use std::num::NonZeroUsize;

mod evaluation;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "256")]
    width: NonZeroUsize,
    #[arg(long, default_value = "1000")]
    length: NonZeroUsize,
    #[arg(long, default_value = "7")]
    sample: NonZeroUsize,
    #[arg(long, default_value_t = 0)]
    noise: usize,
    #[arg(long)]
    changing: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let particle = (0..argument.width.get())
        .map(|index| format!("Value{index}"))
        .collect::<Vec<_>>()
        .join(".");
    let content = std::iter::once(particle.clone())
        .chain((0..argument.noise).map(|index| format!("Noise{index}")))
        .collect::<Vec<_>>()
        .join(".");
    let world = |stage| {
        if argument.changing {
            format!("Stage{stage}.{content},B")
        } else {
            format!("{content},Stage{stage}.B")
        }
    };
    let mut source = format!("{}\n[{particle},B] Never\n", world(0));
    for index in 0..argument.length.get() {
        source.push_str(&format!("[Stage{index}] Stage{}\n", index + 1));
    }
    let program = photonic::lowering::parse(&source)?;
    let target = photonic::lowering::parse(&world(argument.length.get()))?;
    let limit = Limit {
        state: argument.length.get() + 1,
        record: 100_000_000,
        world: 2,
        cell: argument.width.get() + argument.noise + 2,
        frame: 1,
    };
    evaluation::warm(&program, &target, Some(limit));
    let measurement = (0..argument.sample.get())
        .map(|_| evaluation::evaluate(program.clone(), target.clone(), Some(limit)))
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width, "length": argument.length, "noise": argument.noise, "changing": argument.changing, "measurement": measurement,
        }),
    )?;
    Ok(())
}
