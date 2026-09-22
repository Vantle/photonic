use clap::Parser;
use std::num::NonZeroUsize;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "64")]
    width: NonZeroUsize,
    #[arg(long, default_value = "64")]
    depth: NonZeroUsize,
    #[arg(long, default_value = "64")]
    length: NonZeroUsize,
    #[arg(long, default_value = "1")]
    stride: NonZeroUsize,
    #[arg(long, default_value = "9")]
    sample: NonZeroUsize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let run = || {
        photonic::measurement::scope::run(
            argument.width.get(),
            argument.depth.get(),
            argument.length.get(),
            argument.stride.get(),
        )
    };
    run();
    let measurement = (0..argument.sample.get())
        .map(|_| run())
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width,
            "depth": argument.depth,
            "length": argument.length,
            "stride": argument.stride,
            "measurement": measurement,
        }),
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "test/scope.rs"]
mod test;
