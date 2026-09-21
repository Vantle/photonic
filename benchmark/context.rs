use clap::Parser;
use std::num::NonZeroUsize;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "12")]
    width: usize,
    #[arg(long, default_value = "32")]
    length: NonZeroUsize,
    #[arg(long, default_value = "9")]
    sample: NonZeroUsize,
    #[arg(long)]
    productive: bool,
    #[arg(long)]
    scalar: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    if !(8..=16).contains(&argument.width) {
        return Err("require 8 <= width <= 16".into());
    }
    let measurement =
        photonic::measurement::context::run(photonic::measurement::context::Configuration {
            width: argument.width,
            length: argument.length.get(),
            sample: argument.sample.get(),
            productive: argument.productive,
            scalar: argument.scalar,
        });
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width, "length": argument.length, "productive": argument.productive, "scalar": argument.scalar, "measurement": measurement,
        }),
    )?;
    Ok(())
}
