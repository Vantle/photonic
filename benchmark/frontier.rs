use clap::Parser;
use std::num::NonZeroUsize;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "12")]
    width: NonZeroUsize,
    #[arg(long, default_value = "32")]
    count: NonZeroUsize,
    #[arg(long, default_value = "16384")]
    frontier: NonZeroUsize,
    #[arg(long, default_value = "8192")]
    length: NonZeroUsize,
    #[arg(long, default_value = "9")]
    sample: NonZeroUsize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    if argument.count.get() < 2 {
        return Err("require count >= 2".into());
    }
    let measurement =
        photonic::measurement::sharing::partial(photonic::measurement::sharing::Configuration {
            width: argument.width.get(),
            count: argument.count.get(),
            frontier: argument.frontier.get(),
            length: argument.length.get(),
            sample: argument.sample.get(),
        });
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "width": argument.width, "count": argument.count, "frontier": argument.frontier,
            "length": argument.length, "measurement": measurement,
        }),
    )?;
    Ok(())
}
