use clap::Parser;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "4096")]
    width: NonZeroUsize,
    #[arg(long, default_value = "32")]
    length: NonZeroUsize,
    #[arg(long, default_value = "21")]
    sample: NonZeroUsize,
    #[arg(long)]
    descending: bool,
    #[arg(long)]
    replacement: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let run = || {
        photonic::measurement::layout::run(
            argument.width.get(),
            argument.length.get(),
            argument.descending,
            argument.replacement,
        )
    };
    let warm = Instant::now();
    while warm.elapsed() < Duration::from_millis(100) {
        run();
    }
    let measurement = (0..argument.sample.get())
        .map(|_| run())
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(std::io::stdout().lock(), &measurement)?;
    Ok(())
}
