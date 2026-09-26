use clap::Parser;
use photonic::runtime::{Limit, Runtime};
use std::hint::black_box;
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "32")]
    length: NonZeroUsize,
    #[arg(long, default_value = "1")]
    interval: NonZeroUsize,
    #[arg(long, default_value = "7")]
    sample: NonZeroUsize,
}

fn evaluate(program: &frontend::source::Program, argument: &Argument) -> serde_json::Value {
    let target = frontend::lowering::parse("Stage0").unwrap();
    let mut runtime = Runtime::new(program);
    let limit = Limit {
        configuration: argument.length.get() + 1,
        record: 100_000_000,
        coherence: 1,
        occurrence: program.rule.len() + 1,
        scope: 1,
    };
    let start = Instant::now();
    let mut remaining = 40000;
    while remaining > 0 {
        let budget = argument.interval.get().min(remaining);
        runtime.run(budget, limit);
        black_box(runtime.verdict(&target));
        remaining -= budget;
    }
    let execution = start.elapsed().as_secs_f64();
    let snapshot = runtime.snapshot();
    assert!(snapshot.closed);
    assert_eq!(snapshot.state.len(), argument.length.get() + 1);
    serde_json::json!({
        "execution": execution,
        "state": snapshot.state.len(),
        "event": snapshot.event.len(),
        "work": snapshot.work,
        "record": snapshot.record,
        "peak": snapshot.peak,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let mut source = String::from("Stage0,\n");
    for stage in 0..argument.length.get() {
        source.push_str(&format!("[Stage{stage}] Stage{},\n", stage + 1));
    }
    let program = frontend::lowering::parse(&source)?;
    let warm = Instant::now();
    while warm.elapsed() < Duration::from_millis(100) {
        evaluate(&program, &argument);
    }
    let measurement = (0..argument.sample.get())
        .map(|_| evaluate(&program, &argument))
        .collect::<Vec<_>>();
    serde_json::to_writer_pretty(
        std::io::stdout().lock(),
        &serde_json::json!({
            "length": argument.length,
            "interval": argument.interval,
            "measurement": measurement,
        }),
    )?;
    Ok(())
}
