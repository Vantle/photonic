use std::hint::black_box;
use std::time::Instant;

use clap::Parser;
use photonic::executor::Executor;
use photonic::laser::Laser;
use photonic::runtime::{Limit, Runtime};
use serde::{Deserialize, Serialize};

mod directory;

#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value_t = 3)]
    sample: usize,
    #[arg(long, default_value_t = 100_000_000)]
    budget: usize,
    #[arg(long, default_value_t = 10.0)]
    patience: f64,
    #[arg(
        long,
        help = "Measure only this program, on Laser alone: Photonic source, or a .json program assembled by Bazel"
    )]
    program: Option<std::path::PathBuf>,
    #[arg(long, default_value_t = usize::MAX, help = "Configurations Laser keeps")]
    configuration: usize,
    #[arg(long, default_value_t = Limit::default().occurrence, help = "Occurrences in one configuration")]
    occurrence: usize,
    #[arg(
        long,
        value_enum,
        default_value_t = Mode::Exhaustive,
        requires = "program",
        help = "Explore every future, every schedule of plain events, or those schedules one commuting part at a time"
    )]
    mode: Mode,
    #[arg(long, default_value_t = std::num::NonZeroUsize::MIN, help = "Threads Laser explores with")]
    worker: std::num::NonZeroUsize,
    #[arg(long, help = "Measure only cases whose names contain this text")]
    filter: Option<String>,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Mode {
    Exhaustive,
    Plain,
    Reduced,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    program: frontend::source::Program,
    closed: bool,
}

#[derive(Serialize)]
struct Engine {
    closed: bool,
    state: usize,
    event: usize,
    work: usize,
    median: f64,
}

#[derive(Serialize)]
struct Measurement {
    name: String,
    laser: Engine,
    interpreter: Option<Engine>,
    agree: Option<bool>,
}

struct Run {
    closed: bool,
    state: usize,
    event: usize,
    work: usize,
    second: f64,
}

fn laser(
    program: &frontend::source::Program,
    mode: Mode,
    executor: &Executor,
    budget: usize,
    limit: Limit,
) -> Run {
    let start = Instant::now();
    let mut laser = match mode {
        Mode::Exhaustive => Laser::new(black_box(program)),
        Mode::Plain => Laser::plain(black_box(program)),
        Mode::Reduced => Laser::reduced(black_box(program)),
    };
    laser.parallel(executor, budget, limit);
    let summary = black_box(laser.summary());
    Run {
        closed: summary.closed,
        state: summary.state,
        event: summary.event,
        work: summary.work,
        second: start.elapsed().as_secs_f64(),
    }
}

fn interpreter(program: &frontend::source::Program, budget: usize, limit: Limit) -> Run {
    let start = Instant::now();
    let mut runtime = Runtime::new(black_box(program));
    runtime.run(budget, limit);
    let snapshot = black_box(runtime.snapshot());
    Run {
        closed: snapshot.closed,
        state: snapshot.state.len(),
        event: snapshot.event.len(),
        work: snapshot.work,
        second: start.elapsed().as_secs_f64(),
    }
}

fn measure(sample: usize, patience: f64, mut run: impl FnMut() -> Run) -> Engine {
    let mut second = Vec::new();
    let mut last = None;
    for _ in 0..sample {
        let value = run();
        second.push(value.second);
        let slow = value.second > patience;
        last = Some(value);
        if slow {
            break;
        }
    }
    second.sort_by(f64::total_cmp);
    let last = last.expect("at least one sample");
    Engine {
        closed: last.closed,
        state: last.state,
        event: last.event,
        work: last.work,
        median: second[second.len() / 2],
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    directory::enter()?;
    let argument = Argument::parse();
    let executor = Executor::new(argument.worker)?;
    let limit = Limit {
        record: usize::MAX,
        configuration: argument.configuration,
        occurrence: argument.occurrence,
        ..Limit::default()
    };
    if let Some(path) = &argument.program {
        let text = std::fs::read_to_string(path)?;
        let program = if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            frontend::source::Program::read(&text)?
        } else {
            frontend::lowering::parse(&text)?
        };
        #[cfg(feature = "measurement")]
        photonic::profile::take();
        let engine = measure(argument.sample, argument.patience, || {
            laser(&program, argument.mode, &executor, argument.budget, limit)
        });
        #[cfg(feature = "measurement")]
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&photonic::profile::take())?
        );
        println!("{}", serde_json::to_string_pretty(&engine)?);
        return Ok(());
    }
    let mut case =
        serde_json::from_str::<Vec<Case>>(include_str!("../language/test/reference.json"))?
            .into_iter()
            .filter(|case| case.closed)
            .map(|case| (case.name, case.program, true))
            .collect::<Vec<_>>();
    for count in [4, 6, 8, 10, 12] {
        case.push((
            format!("{count} dials"),
            frontend::lowering::parse(&photonic::family::dial(count))?,
            count <= 8,
        ));
    }
    for count in [3, 4, 5, 6] {
        case.push((
            format!("{count} diners"),
            frontend::lowering::parse(&photonic::family::diner(count))?,
            count <= 4,
        ));
    }
    case.retain(|(name, _, _)| {
        argument
            .filter
            .as_ref()
            .is_none_or(|filter| name.contains(filter.as_str()))
    });
    let mut report = Vec::new();
    for (name, program, compare) in case {
        let laser = measure(argument.sample, argument.patience, || {
            laser(
                &program,
                Mode::Exhaustive,
                &executor,
                argument.budget,
                limit,
            )
        });
        let interpreter = compare.then(|| {
            measure(argument.sample, argument.patience, || {
                interpreter(&program, argument.budget, limit)
            })
        });
        let agree = interpreter
            .as_ref()
            .filter(|interpreter| interpreter.closed && laser.closed)
            .map(|interpreter| {
                interpreter.state == laser.state && interpreter.event == laser.event
            });
        report.push(Measurement {
            name,
            laser,
            interpreter,
            agree,
        });
    }
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
