use clap::Parser;
use photonic::runtime::{Limit, Runtime};
use std::fmt;
use std::hint::black_box;
use std::num::NonZeroUsize;
use std::process::ExitCode;
use std::time::Instant;

mod exit;
mod warm;

#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value = "32")]
    length: NonZeroUsize,
    #[arg(long, default_value = "1")]
    interval: NonZeroUsize,
    #[arg(long, default_value = "7")]
    sample: NonZeroUsize,
    #[arg(long, default_value_t = 40_000, help = "Work steps each run takes")]
    budget: usize,
}

#[derive(Debug)]
enum Failure {
    Open,
    Length { expected: usize, actual: usize },
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => {
                formatter.write_str("the pipeline did not close within the budget; raise --budget")
            }
            Self::Length { expected, actual } => write!(
                formatter,
                "the pipeline reached {actual} configurations instead of {expected}"
            ),
        }
    }
}

impl std::error::Error for Failure {}

fn evaluate(
    program: &frontend::source::Program,
    target: &frontend::source::Program,
    argument: &Argument,
) -> Result<serde_json::Value, Failure> {
    let mut runtime = Runtime::new(program);
    let limit = Limit {
        configuration: argument.length.get() + 1,
        record: 100_000_000,
        coherence: 1,
        occurrence: program.rule.len() + 1,
        scope: 1,
    };
    let start = Instant::now();
    let mut remaining = argument.budget;
    while remaining > 0 {
        let budget = argument.interval.get().min(remaining);
        runtime.run(budget, limit);
        black_box(runtime.verdict(target));
        remaining -= budget;
    }
    let execution = start.elapsed().as_secs_f64();
    let snapshot = runtime.snapshot();
    if !snapshot.closed {
        return Err(Failure::Open);
    }
    let expected = argument.length.get() + 1;
    if snapshot.state.len() != expected {
        return Err(Failure::Length {
            expected,
            actual: snapshot.state.len(),
        });
    }
    Ok(serde_json::json!({
        "execution": execution,
        "state": snapshot.state.len(),
        "event": snapshot.event.len(),
        "work": snapshot.work,
        "record": snapshot.record,
        "peak": snapshot.peak,
    }))
}

fn main() -> ExitCode {
    exit::code(run(&Argument::parse()))
}

fn run(argument: &Argument) -> Result<(), Box<dyn std::error::Error>> {
    let mut source = String::from("Stage0,\n");
    for stage in 0..argument.length.get() {
        source.push_str(&format!("[Stage{stage}] Stage{},\n", stage + 1));
    }
    let program = frontend::lowering::parse(&source)?;
    let target = frontend::lowering::parse("Stage0")?;
    warm::warm(|| evaluate(&program, &target, argument).map(drop))?;
    let measurement = (0..argument.sample.get())
        .map(|_| evaluate(&program, &target, argument))
        .collect::<Result<Vec<_>, _>>()?;
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
