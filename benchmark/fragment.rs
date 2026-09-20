use clap::Parser;
use photonic::runtime::Limit;
use std::num::NonZeroUsize;

mod evaluation;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "64")]
    width: NonZeroUsize,
    #[arg(long, default_value = "256")]
    size: NonZeroUsize,
    #[arg(long, default_value = "100")]
    length: NonZeroUsize,
    #[arg(long, default_value = "7")]
    sample: NonZeroUsize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let particle = vec!["A"; argument.size.get()].join(".");
    let context = (0..argument.width.get())
        .map(|index| format!("Branch{index}"))
        .collect::<Vec<_>>()
        .join(",");
    let mut source = format!("Stage0.{particle},{context}\n");
    for index in 0..argument.width.get() {
        source.push_str(&format!("[{particle}.A,Branch{index}] Never{index}\n"));
    }
    for index in 0..argument.length.get() {
        source.push_str(&format!("[Stage{index}] Stage{}\n", index + 1));
    }
    let program = photonic::lowering::parse(&source)?;
    let target =
        photonic::lowering::parse(&format!("Stage{}.{particle},{context}", argument.length))?;
    let limit = Limit {
        state: argument.length.get() + 1,
        record: 100_000_000,
        world: argument.width.get() + 1,
        cell: argument.width.get() + argument.size.get() + 1,
        frame: 1,
    };
    evaluation::warm(&program, &target, Some(limit));
    let measurement = (0..argument.sample.get())
        .map(|_| evaluation::evaluate(program.clone(), target.clone(), Some(limit)))
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width, "size": argument.size,
            "length": argument.length, "measurement": measurement,
        }),
    )?;
    Ok(())
}
