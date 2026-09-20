use clap::Parser;
use std::num::NonZeroUsize;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "4096")]
    width: NonZeroUsize,
    #[arg(long, default_value = "256")]
    length: NonZeroUsize,
    #[arg(long, default_value = "9")]
    sample: NonZeroUsize,
    #[arg(long)]
    private: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let run = || {
        photonic::measurement::preparation::run(
            argument.width.get(),
            argument.length.get(),
            !argument.private,
        )
    };
    run();
    let measurement = (0..argument.sample.get())
        .map(|_| run())
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width, "length": argument.length, "shared": !argument.private, "measurement": measurement,
        }),
    )?;
    Ok(())
}
