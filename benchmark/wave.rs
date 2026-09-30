use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use photonic::executor::Executor;
use photonic::laser::Laser;
use photonic::laser::net::{Cycle, Net};
use photonic::runtime::Limit;
use serde::Serialize;

mod directory;
mod program;
mod statistic;

#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Parser)]
struct Argument {
    #[arg(
        long,
        required_unless_present = "family",
        conflicts_with = "family",
        help = "Photonic source, or a .json program assembled by Bazel, to explore every schedule of plain events of"
    )]
    program: Option<PathBuf>,
    #[arg(
        long,
        value_enum,
        requires = "count",
        help = "A family of programs that grows as large as wanted"
    )]
    family: Option<Family>,
    #[arg(
        long,
        help = "How many dials, diners or tasks the family's program has"
    )]
    count: Option<usize>,
    #[arg(long, default_value_t = usize::MAX, help = "Configurations kept")]
    configuration: usize,
    #[arg(long, default_value_t = Limit::default().occurrence, help = "Occurrences in one configuration")]
    occurrence: usize,
    #[arg(long, default_value = "3", help = "Runs of each engine")]
    sample: NonZeroUsize,
    #[arg(
        long,
        help = "Also explore with the net on the host and with Laser's plain engine, and check that the net agrees"
    )]
    compare: bool,
    #[arg(long, default_value_t = NonZeroUsize::MIN, help = "Threads Laser's plain engine explores with")]
    worker: NonZeroUsize,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Family {
    Dial,
    Diner,
    Task,
}

#[derive(Serialize)]
struct Throughput {
    closed: bool,
    configuration: usize,
    event: u64,
    second: statistic::Spread,
    rate: f64,
}

#[derive(Serialize)]
struct Measurement {
    device: String,
    metal: Throughput,
    net: Option<Throughput>,
    plain: Option<Throughput>,
    agree: Option<String>,
}

fn throughput(
    closed: bool,
    configuration: usize,
    event: u64,
    sample: &statistic::Sample,
) -> Throughput {
    let second = sample.spread();
    Throughput {
        closed,
        configuration,
        event,
        rate: configuration as f64 / second.median,
        second,
    }
}

fn program(argument: &Argument) -> Result<frontend::source::Program, Box<dyn std::error::Error>> {
    if let (Some(family), Some(count)) = (argument.family, argument.count) {
        let text = match family {
            Family::Dial => photonic::family::dial(count),
            Family::Diner => photonic::family::diner(count),
            Family::Task => photonic::family::task(count),
        };
        return Ok(frontend::lowering::parse(&text)?);
    }
    program::read(argument.program.as_ref().ok_or("a program or a family")?)
}

// Every engine is built before its timer starts, so each sample times the exploration alone.
fn sample<Value, Failure>(
    count: NonZeroUsize,
    mut run: impl FnMut() -> Result<(Value, f64), Failure>,
) -> Result<(Value, statistic::Sample), Failure> {
    let (mut last, second) = run()?;
    let mut sample = statistic::Sample::new(second);
    for _ in 1..count.get() {
        let (value, second) = run()?;
        sample.push(second);
        last = value;
    }
    Ok((last, sample))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    directory::enter()?;
    let argument = Argument::parse();
    let program = program(&argument)?;
    let limit = Limit {
        record: usize::MAX,
        configuration: argument.configuration,
        occurrence: argument.occurrence,
        ..Limit::default()
    };
    let device = wave::engine::Engine::new()?.ok_or("no Metal device to explore on")?;
    let ((explored, net), metal) = sample(argument.sample, || {
        let mut net = Net::new(&program)?;
        let start = Instant::now();
        let explored = device.explore(&mut net, usize::MAX, limit, Cycle::Ignore)?;
        let second = start.elapsed().as_secs_f64();
        Ok::<_, Box<dyn std::error::Error>>(((explored, net), second))
    })?;
    let mut measurement = Measurement {
        device: device.name().to_owned(),
        metal: throughput(
            explored.closed,
            explored.configuration,
            explored.event,
            &metal,
        ),
        net: None,
        plain: None,
        agree: None,
    };
    if argument.compare {
        let ((expected, theirs), host) = sample(argument.sample, || {
            let mut theirs = Net::new(&program)?;
            let start = Instant::now();
            let expected = theirs.explore(usize::MAX, limit, Cycle::Ignore)?;
            let second = start.elapsed().as_secs_f64();
            Ok::<_, Box<dyn std::error::Error>>(((expected, theirs), second))
        })?;
        measurement.agree = Some(match explored.agrees(&net, &expected, &theirs) {
            Ok(()) => "agrees".to_owned(),
            Err(disagreement) => format!("{disagreement:?}"),
        });
        measurement.net = Some(throughput(
            expected.closed,
            expected.configuration,
            expected.event,
            &host,
        ));
        let executor = Executor::new(argument.worker)?;
        let (laser, plain) = sample(argument.sample, || {
            let mut laser = Laser::plain(&program);
            let start = Instant::now();
            laser.parallel(&executor, usize::MAX, limit);
            let second = start.elapsed().as_secs_f64();
            Ok::<_, Box<dyn std::error::Error>>((laser, second))
        })?;
        let summary = laser.summary();
        measurement.plain = Some(throughput(
            summary.closed,
            summary.state,
            summary.event as u64,
            &plain,
        ));
    }
    println!("{}", serde_json::to_string_pretty(&measurement)?);
    Ok(())
}
