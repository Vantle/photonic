use clap::Parser;
use std::num::NonZeroUsize;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "3")]
    width: NonZeroUsize,
    #[arg(long, default_value = "32768")]
    length: NonZeroUsize,
    #[arg(long, default_value = "9")]
    sample: NonZeroUsize,
    #[arg(long)]
    productive: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let run = || {
        photonic::measurement::residual::run(
            argument.width.get(),
            argument.length.get(),
            argument.productive,
        )
    };
    run();
    let measurement = (0..argument.sample.get())
        .map(|_| run())
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width, "length": argument.length, "productive": argument.productive, "measurement": measurement,
        }),
    )?;
    Ok(())
}
