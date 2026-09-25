use clap::Parser;
use photonic::prism::Search;
use photonic::runtime::Limit;
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

fn evaluate(program: &photonic::source::Program, argument: &Argument) -> serde_json::Value {
    let mut search = Search::new(
        program.clone(),
        photonic::lowering::parse("Stage0").unwrap(),
    );
    let limit = Limit {
        state: argument.length.get() + 1,
        record: 100_000_000,
        world: 1,
        cell: program.rule.len() + 1,
        frame: 1,
    };
    let start = Instant::now();
    let mut remaining = 40000;
    while remaining > 0 {
        let budget = argument.interval.get().min(remaining);
        search.run(budget, Some(limit));
        black_box(search.verdict());
        remaining -= budget;
    }
    let execution = start.elapsed().as_secs_f64();
    let report = search.report();
    assert!(report.execution.closed);
    assert_eq!(report.execution.state.len(), argument.length.get() + 1);
    serde_json::json!({
        "execution": execution,
        "state": report.execution.state.len(),
        "event": report.execution.event.len(),
        "work": report.execution.work,
        "record": report.execution.record,
        "peak": report.execution.peak,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argument = Argument::parse();
    let mut source = String::from("Stage0,\n");
    for stage in 0..argument.length.get() {
        source.push_str(&format!("[Stage{stage}] Stage{},\n", stage + 1));
    }
    let program = photonic::lowering::parse(&source)?;
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
