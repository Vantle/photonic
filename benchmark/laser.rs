use std::hint::black_box;
use std::num::NonZeroUsize;
use std::time::Instant;

use clap::Parser;
use photonic::executor::Executor;
use photonic::laser::Laser;
use photonic::runtime::{Limit, Runtime};
use photonic::snapshot::Snapshot;
use serde::Serialize;

mod directory;
mod program;
mod reference;
mod statistic;

#[global_allocator]
static ALLOCATOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

// The interpreter takes minutes and tens of gigabytes past six dials or four diners, and Laser
// seconds and gigabytes past ten dials or seven diners, so the families stop there.
const DIAL: [(usize, bool); 5] = [(4, true), (6, true), (8, false), (9, false), (10, false)];
const DINER: [(usize, bool); 5] = [(3, true), (4, true), (5, false), (6, false), (7, false)];

#[derive(Parser)]
struct Argument {
    #[arg(
        long,
        default_value = "3",
        help = "Runs of each engine on each program"
    )]
    sample: NonZeroUsize,
    #[arg(long, default_value_t = 100_000_000)]
    budget: usize,
    #[arg(
        long,
        default_value_t = 10.0,
        help = "Seconds after which a run is the last one sampled"
    )]
    patience: f64,
    #[arg(
        long,
        help = "Measure only this program, on Laser alone: Photonic source, or a .json program assembled by Bazel"
    )]
    program: Option<std::path::PathBuf>,
    #[arg(long, default_value_t = 65_536, help = "Configurations Laser keeps")]
    configuration: usize,
    #[arg(
        long,
        default_value_t = 100_000_000,
        help = "Records each engine retains"
    )]
    record: usize,
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
    #[arg(long, default_value_t = NonZeroUsize::MIN, help = "Threads Laser explores with")]
    worker: NonZeroUsize,
    #[arg(long, help = "Measure only cases whose names contain this text")]
    filter: Option<String>,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Mode {
    Exhaustive,
    Plain,
    Reduced,
}

#[derive(Serialize)]
struct Summary {
    closed: bool,
    state: usize,
    event: usize,
    work: usize,
    second: statistic::Spread,
}

#[derive(Serialize)]
struct Measurement {
    name: String,
    laser: Summary,
    interpreter: Option<Summary>,
    verdict: Option<String>,
}

fn laser(
    program: &frontend::source::Program,
    mode: Mode,
    executor: &Executor,
    budget: usize,
    limit: Limit,
) -> (Laser, f64) {
    let start = Instant::now();
    let mut laser = match mode {
        Mode::Exhaustive => Laser::new(black_box(program)),
        Mode::Plain => Laser::plain(black_box(program)),
        Mode::Reduced => Laser::reduced(black_box(program)),
    };
    laser.parallel(executor, budget, limit);
    black_box(laser.summary());
    (laser, start.elapsed().as_secs_f64())
}

fn interpreter(
    program: &frontend::source::Program,
    budget: usize,
    limit: Limit,
) -> ((Runtime, Snapshot), f64) {
    let start = Instant::now();
    let mut runtime = Runtime::new(black_box(program));
    runtime.run(budget, limit);
    let snapshot = black_box(runtime.snapshot());
    ((runtime, snapshot), start.elapsed().as_secs_f64())
}

// A run slower than the patience ends the sample, so a slow program is measured fewer times
// instead of holding up the rest; the last run is kept to report and compare.
fn measure<Value>(
    count: NonZeroUsize,
    patience: f64,
    mut run: impl FnMut() -> (Value, f64),
) -> (Value, statistic::Sample) {
    let (mut last, second) = run();
    let mut sample = statistic::Sample::new(second);
    let mut slow = second > patience;
    for _ in 1..count.get() {
        if slow {
            break;
        }
        let (value, second) = run();
        sample.push(second);
        slow = second > patience;
        last = value;
    }
    (last, sample)
}

fn summary(laser: &Laser, sample: &statistic::Sample) -> Summary {
    let summary = laser.summary();
    Summary {
        closed: summary.closed,
        state: summary.state,
        event: summary.event,
        work: summary.work,
        second: sample.spread(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    directory::enter()?;
    let argument = Argument::parse();
    let executor = Executor::new(argument.worker)?;
    let limit = Limit {
        record: argument.record,
        configuration: argument.configuration,
        occurrence: argument.occurrence,
        ..Limit::default()
    };
    if let Some(path) = &argument.program {
        let program = program::read(path)?;
        #[cfg(feature = "measurement")]
        photonic::profile::take();
        let (laser, sample) = measure(argument.sample, argument.patience, || {
            laser(&program, argument.mode, &executor, argument.budget, limit)
        });
        #[cfg(feature = "measurement")]
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&photonic::profile::take())?
        );
        println!(
            "{}",
            serde_json::to_string_pretty(&summary(&laser, &sample))?
        );
        return Ok(());
    }
    let mut case = reference::load()?
        .into_iter()
        .filter(|case| case.closed)
        .map(|case| (case.name, case.program, true))
        .collect::<Vec<_>>();
    for (count, compare) in DIAL {
        case.push((
            format!("{count} dials"),
            frontend::lowering::parse(&photonic::family::dial(count))?,
            compare,
        ));
    }
    for (count, compare) in DINER {
        case.push((
            format!("{count} diners"),
            frontend::lowering::parse(&photonic::family::diner(count))?,
            compare,
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
        let (laser, sample) = measure(argument.sample, argument.patience, || {
            laser(
                &program,
                Mode::Exhaustive,
                &executor,
                argument.budget,
                limit,
            )
        });
        let measured = compare.then(|| {
            measure(argument.sample, argument.patience, || {
                interpreter(&program, argument.budget, limit)
            })
        });
        let verdict = measured
            .as_ref()
            .filter(|((runtime, _), _)| runtime.closed() && laser.closed())
            .map(|((runtime, _), _)| match laser.agree(runtime) {
                Ok(()) => "agree".to_owned(),
                Err(disagreement) => format!("{disagreement:?}"),
            });
        let interpreter = measured.map(|((_, snapshot), sample)| Summary {
            closed: snapshot.closed,
            state: snapshot.state.len(),
            event: snapshot.event.len(),
            work: snapshot.work,
            second: sample.spread(),
        });
        report.push(Measurement {
            name,
            laser: summary(&laser, &sample),
            interpreter,
            verdict,
        });
    }
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
