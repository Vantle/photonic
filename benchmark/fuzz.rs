use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::process::ExitCode;

use clap::Parser;
use photonic::executor::Executor;
use photonic::laser::Laser;
use photonic::laser::net::{Cycle, Net};
use photonic::runtime::{Limit, Runtime};
use random::Generator;
use rayon::prelude::*;
use serde::Serialize;

mod directory;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value_t = 0, help = "The first program's seed")]
    seed: u64,
    #[arg(
        long,
        default_value_t = 1000,
        help = "Programs written, one for each seed"
    )]
    count: u64,
    #[arg(long, default_value_t = 200_000, help = "Interpreter work steps")]
    budget: usize,
    #[arg(long, default_value_t = 500_000, help = "Laser and net work steps")]
    allowance: usize,
    #[arg(long, default_value_t = 256, help = "Configurations kept")]
    configuration: usize,
    #[arg(long, default_value_t = 100_000, help = "Records an engine retains")]
    record: usize,
    #[arg(
        long,
        help = "Also explore each program on this many workers and require the same report; a check waiting on the workers runs other programs meanwhile, so memory grows"
    )]
    worker: Option<NonZeroUsize>,
    #[arg(long, help = "Print each written program instead of checking it")]
    write: bool,
}

const ATOM: [&str; 4] = ["A", "B", "C", "D"];

// Comparing two whole reports holds both in memory at once, so the reports compared stay small.
const EVENT: usize = 10_000;

// Writes programs from the language's own forms: coherences of dotted atoms, rules with one to
// three inputs, empty inputs and inputs that hold a rule value, empty, single, grouped and scoped
// outputs, outputs that carry rules in an empty particle or in scopes of their own, rule values
// held as fields and scopes the program opens, over few atoms so that rules meet what others make.
struct Writer {
    generator: Generator,
}

impl Writer {
    fn atom(&mut self) -> &'static str {
        ATOM[self.generator.below(ATOM.len())]
    }

    fn particle(&mut self) -> String {
        let length = 1 + self.generator.below(3);
        (0..length)
            .map(|_| self.atom())
            .collect::<Vec<_>>()
            .join(".")
    }

    fn input(&mut self, depth: usize) -> String {
        match self.generator.below(20) {
            0 | 1 => "()".to_owned(),
            2 if depth > 0 => format!("{}.({})", self.particle(), self.rule(depth - 1)),
            _ => self.particle(),
        }
    }

    fn output(&mut self, depth: usize) -> String {
        match self.generator.below(if depth == 0 { 5 } else { 9 }) {
            0 => String::new(),
            1 => "()".to_owned(),
            2 | 3 => self.particle(),
            4 => format!("({}, {})", self.particle(), self.particle()),
            5 => format!("().({})", self.rule(depth - 1)),
            6 => format!("(({}), ({}))", self.rule(depth - 1), self.rule(depth - 1)),
            _ => format!("({})", self.program(depth - 1)),
        }
    }

    fn rule(&mut self, depth: usize) -> String {
        let count =
            1 + usize::from(self.generator.chance(0.3)) + usize::from(self.generator.chance(0.1));
        let input = (0..count)
            .map(|_| self.input(depth))
            .collect::<Vec<_>>()
            .join(", ");
        let output = self.output(depth);
        if output.is_empty() {
            return format!("[{input}]");
        }
        format!("[{input}] {output}")
    }

    fn term(&mut self, depth: usize) -> String {
        match self.generator.below(if depth == 0 { 9 } else { 10 }) {
            0..=2 => self.particle(),
            3 => format!("{}.({})", self.particle(), self.rule(0)),
            4 => format!(
                "{}.(({}), ({}))",
                self.particle(),
                self.rule(0),
                self.rule(0)
            ),
            5..=8 => self.rule(depth),
            _ => format!("({})", self.program(depth - 1)),
        }
    }

    fn program(&mut self, depth: usize) -> String {
        let count = 2 + self.generator.below(4);
        (0..count)
            .map(|_| self.term(depth))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn write(seed: u64) -> String {
    Writer {
        generator: Generator::new(seed),
    }
    .program(1)
}

#[derive(Serialize)]
struct Finding {
    seed: u64,
    check: &'static str,
    detail: String,
    source: String,
}

// The checks one program took and the disagreements they found.
#[derive(Default)]
struct Outcome {
    check: Vec<&'static str>,
    finding: Vec<Finding>,
}

impl Outcome {
    fn record(&mut self, check: &'static str, result: Result<(), String>, seed: u64, source: &str) {
        self.check.push(check);
        if let Err(detail) = result {
            self.finding.push(Finding {
                seed,
                check,
                detail,
                source: source.to_owned(),
            });
        }
    }
}

// Every check the census makes, on one written program: Laser against the interpreter where both
// close, also after resuming under raised limits, Laser on several workers against Laser on one
// when asked, the plain exploration within the full one, the reduced exploration against the
// plain one, the net against the plain exploration, and the GPU against the host's net.
fn check(
    seed: u64,
    argument: &Argument,
    executor: Option<&Executor>,
    device: Option<&wave::engine::Engine>,
) -> Option<Outcome> {
    let source = write(seed);
    let program = frontend::lowering::parse(&source).ok()?;
    let limit = Limit {
        configuration: argument.configuration,
        record: argument.record,
        ..Limit::default()
    };
    let text = |disagreement| format!("{disagreement:?}");
    let mut outcome = Outcome::default();
    let mut runtime = Runtime::new(&program);
    runtime.run(argument.budget, limit);
    let mut laser = Laser::new(&program);
    laser.run(argument.allowance, limit);
    if runtime.closed() && laser.closed() {
        outcome.record("laser", laser.agree(&runtime).map_err(text), seed, &source);
    }
    let resumed = resume(&program, argument.allowance, limit);
    if runtime.closed() && resumed.closed() {
        outcome.record(
            "resume",
            resumed.agree(&runtime).map_err(text),
            seed,
            &source,
        );
    }
    if let Some(executor) = executor
        && laser.summary().event <= EVENT
    {
        let mut parallel = Laser::new(&program);
        parallel.parallel(executor, argument.allowance, limit);
        outcome.record("worker", same(&laser, &parallel), seed, &source);
    }
    let mut plain = Laser::plain(&program);
    plain.run(argument.allowance, limit);
    if laser.closed() {
        outcome.record("plain", plain.within(&laser).map_err(text), seed, &source);
    }
    if plain.closed() {
        let mut reduced = Laser::reduced(&program);
        reduced.run(argument.allowance, limit);
        outcome.record(
            "reduced",
            reduced.preserves(&plain).map_err(text),
            seed,
            &source,
        );
    }
    let Ok(mut net) = Net::new(&program) else {
        return Some(outcome);
    };
    let Ok(explored) = net.explore(argument.allowance, limit, Cycle::Find) else {
        return Some(outcome);
    };
    if plain.closed() {
        outcome.record(
            "net",
            explored.mirrors(&net, &plain).map_err(text),
            seed,
            &source,
        );
    }
    if let Some(device) = device {
        let result = Net::new(&program)
            .map_err(|unsupported| unsupported.to_string())
            .and_then(|mut copy| {
                let found = device
                    .explore(&mut copy, argument.allowance, limit, Cycle::Find)
                    .map_err(|failure| failure.to_string())?;
                found.agrees(&copy, &explored, &net).map_err(text)
            });
        outcome.record("metal", result, seed, &source);
    }
    Some(outcome)
}

// Laser explored a step at a time under limits that block events and then rise, so identities
// wait to be retried while new traces arrive, and then to the end.
fn resume(program: &frontend::source::Program, allowance: usize, limit: Limit) -> Laser {
    let mut laser = Laser::new(program);
    for configuration in [2, 4, 8, 16] {
        for _ in 0..2 {
            laser.run(
                1,
                Limit {
                    configuration,
                    ..limit
                },
            );
        }
    }
    laser.run(allowance, limit);
    laser
}

// Laser's report is the same for any number of workers, open or closed.
fn same(sequential: &Laser, parallel: &Laser) -> Result<(), String> {
    let text =
        |laser: &Laser| serde_json::to_string(&laser.report()).map_err(|error| error.to_string());
    if text(sequential)? == text(parallel)? {
        return Ok(());
    }
    Err(format!(
        "{:?} on one worker, {:?} on several",
        sequential.summary(),
        parallel.summary()
    ))
}

// A check that panics is a finding too, so one program cannot end the census.
fn guard(
    seed: u64,
    argument: &Argument,
    executor: Option<&Executor>,
    device: Option<&wave::engine::Engine>,
) -> Option<Outcome> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        check(seed, argument, executor, device)
    }));
    result.unwrap_or_else(|payload| {
        let detail = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| {
                payload
                    .downcast_ref::<&str>()
                    .map(|text| (*text).to_owned())
            })
            .unwrap_or_default();
        let mut outcome = Outcome::default();
        outcome.record("panic", Err(detail), seed, &write(seed));
        Some(outcome)
    })
}

#[derive(Serialize)]
struct Report {
    written: u64,
    parsed: usize,
    check: BTreeMap<&'static str, usize>,
    finding: Vec<Finding>,
}

fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    directory::enter()?;
    let argument = Argument::parse();
    if argument.write {
        for seed in argument.seed..argument.seed + argument.count {
            println!("{seed}: {}", write(seed));
        }
        return Ok(ExitCode::SUCCESS);
    }
    let device = wave::engine::Engine::new()?;
    let executor = argument.worker.map(Executor::new).transpose()?;
    let outcome = (argument.seed..argument.seed + argument.count)
        .into_par_iter()
        .filter_map(|seed| guard(seed, &argument, executor.as_ref(), device.as_ref()))
        .collect::<Vec<_>>();
    let mut report = Report {
        written: argument.count,
        parsed: outcome.len(),
        check: BTreeMap::new(),
        finding: Vec::new(),
    };
    for value in outcome {
        for check in value.check {
            *report.check.entry(check).or_default() += 1;
        }
        report.finding.extend(value.finding);
    }
    report.finding.sort_by_key(|finding| finding.seed);
    let passed = report.finding.is_empty();
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}
