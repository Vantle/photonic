use clap::Parser;
use std::num::NonZeroUsize;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "8")]
    width: NonZeroUsize,
    #[arg(long, default_value = "4")]
    count: NonZeroUsize,
    #[arg(long, default_value = "1")]
    replacement: NonZeroUsize,
    #[arg(long, default_value = "64")]
    length: NonZeroUsize,
    #[arg(long, default_value = "7")]
    sample: NonZeroUsize,
    #[arg(long)]
    productive: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    if argument.count.get() < 2
        || argument.count > argument.width
        || argument.replacement > argument.count
    {
        return Err("require 2 <= count <= width and replacement <= count".into());
    }
    let measurement =
        photonic::measurement::partition::run(photonic::measurement::partition::Configuration {
            width: argument.width.get(),
            count: argument.count.get(),
            replacement: argument.replacement.get(),
            length: argument.length.get(),
            sample: argument.sample.get(),
            productive: argument.productive,
        });
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width, "count": argument.count, "replacement": argument.replacement,
            "length": argument.length, "productive": argument.productive, "measurement": measurement,
        }),
    )?;
    Ok(())
}
