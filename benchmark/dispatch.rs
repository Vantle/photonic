use clap::Parser;
use std::num::NonZeroUsize;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "256")]
    width: NonZeroUsize,
    #[arg(long, default_value = "32")]
    length: NonZeroUsize,
    #[arg(long, default_value = "9")]
    sample: NonZeroUsize,
    #[arg(long)]
    verify: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    if argument.verify {
        serde_json::to_writer_pretty(
            std::io::stdout().lock(),
            &photonic::measurement::subscription::run(),
        )?;
        return Ok(());
    }
    photonic::measurement::dispatch::run(argument.width.get(), argument.length.get());
    let measurement = (0..argument.sample.get())
        .map(|_| photonic::measurement::dispatch::run(argument.width.get(), argument.length.get()))
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width, "length": argument.length, "measurement": measurement,
        }),
    )?;
    Ok(())
}
