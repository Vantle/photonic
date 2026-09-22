use clap::Parser;
use std::num::NonZeroUsize;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "512")]
    width: NonZeroUsize,
    #[arg(long, default_value = "1")]
    depth: NonZeroUsize,
    #[arg(long, default_value = "64")]
    length: NonZeroUsize,
    #[arg(long, default_value = "21")]
    sample: NonZeroUsize,
    #[arg(long)]
    replacement: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let run = || {
        photonic::measurement::fingerprint::run(
            argument.width.get(),
            argument.depth.get(),
            argument.length.get(),
            argument.replacement,
        )
    };
    run();
    let measurement = (0..argument.sample.get())
        .map(|_| run())
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(std::io::stdout().lock(), &measurement)?;
    Ok(())
}
