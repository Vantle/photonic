use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use photonic::executor::Executor;
use photonic::laser::Laser;
use photonic::laser::net::{Cycle, Net};
use photonic::runtime::Limit;
use serde::Serialize;

mod directory;

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
    #[arg(long, default_value_t = 3)]
    sample: usize,
    #[arg(
        long,
        help = "Also explore with the net on the host and with Laser's plain engine, and check that the net agrees"
    )]
    compare: bool,
    #[arg(long, default_value_t = std::num::NonZeroUsize::MIN, help = "Threads Laser's plain engine explores with")]
    worker: std::num::NonZeroUsize,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Family {
    Dial,
    Diner,
    Task,
}

#[derive(Serialize)]
struct Engine {
    closed: bool,
    configuration: usize,
    event: u64,
    median: f64,
    rate: f64,
}

#[derive(Serialize)]
struct Measurement {
    device: String,
    metal: Engine,
    net: Option<Engine>,
    plain: Option<Engine>,
    agree: Option<String>,
}

fn median(mut second: Vec<f64>) -> f64 {
    second.sort_by(f64::total_cmp);
    second[second.len() / 2]
}

fn engine(closed: bool, configuration: usize, event: u64, second: Vec<f64>) -> Engine {
    let median = median(second);
    Engine {
        closed,
        configuration,
        event,
        median,
        rate: configuration as f64 / median,
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
    let path = argument.program.as_ref().ok_or("a program or a family")?;
    let text = std::fs::read_to_string(path)?;
    if path
        .extension()
        .is_some_and(|extension| extension == "json")
    {
        return Ok(frontend::source::Program::read(&text)?);
    }
    Ok(frontend::lowering::parse(&text)?)
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
    let mut second = Vec::new();
    let mut last = None;
    for _ in 0..argument.sample {
        let mut net = Net::new(&program)?;
        let start = Instant::now();
        let explored = device.explore(&mut net, usize::MAX, limit, Cycle::Ignore)?;
        second.push(start.elapsed().as_secs_f64());
        last = Some((explored, net));
    }
    let (explored, net) = last.ok_or("at least one sample")?;
    let metal = engine(
        explored.closed,
        explored.configuration,
        explored.event,
        second,
    );
    let mut measurement = Measurement {
        device: device.name().to_owned(),
        metal,
        net: None,
        plain: None,
        agree: None,
    };
    if argument.compare {
        let mut second = Vec::new();
        let mut last = None;
        for _ in 0..argument.sample {
            let mut theirs = Net::new(&program)?;
            let start = Instant::now();
            let expected = theirs.explore(usize::MAX, limit, Cycle::Ignore)?;
            second.push(start.elapsed().as_secs_f64());
            last = Some((expected, theirs));
        }
        let (expected, theirs) = last.ok_or("at least one sample")?;
        measurement.agree = Some(match explored.agrees(&net, &expected, &theirs) {
            Ok(()) => "agrees".to_owned(),
            Err(disagreement) => format!("{disagreement:?}"),
        });
        measurement.net = Some(engine(
            expected.closed,
            expected.configuration,
            expected.event,
            second,
        ));
        let executor = Executor::new(argument.worker)?;
        let start = Instant::now();
        let mut laser = Laser::plain(&program);
        laser.parallel(&executor, usize::MAX, limit);
        let summary = laser.summary();
        measurement.plain = Some(engine(
            summary.closed,
            summary.state,
            summary.event as u64,
            vec![start.elapsed().as_secs_f64()],
        ));
    }
    println!("{}", serde_json::to_string_pretty(&measurement)?);
    Ok(())
}
