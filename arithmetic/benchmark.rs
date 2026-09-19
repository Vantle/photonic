use arithmetic::{circuit, encoding};
use clap::Parser;
use photonic::lowering::parse;
use photonic::path::Search;
use photonic::prism::Outcome;
use photonic::runtime::Limit;
use std::time::Instant;

#[derive(Parser)]
struct Argument {
    #[arg(long, default_value_t = 1500)]
    left: u64,
    #[arg(long, default_value_t = 123)]
    right: u64,
}

fn measure(radix: u8, operation: &str, source: String, target: String) {
    let mut duration = Vec::new();
    for iteration in 0..6 {
        let start = Instant::now();
        let program = parse(&source).unwrap();
        let count = program.rule.len();
        let mut search = Search::new(program, parse(&target).unwrap()).unwrap();
        search.run(
            20_000_000,
            Limit {
                state: 4096,
                record: 1_000_000,
                cell: 4096,
                frame: 10,
                world: 4,
            },
        );
        let report = search.report();
        assert_eq!(report.outcome, Outcome::Reached);
        if iteration == 0 {
            continue;
        }
        duration.push((
            start.elapsed().as_micros(),
            count,
            report.event.len(),
            report.work,
        ));
    }
    duration.sort();
    println!(
        "radix={radix} operation={operation} microseconds={} rules={} events={} work={} bytes={}",
        duration[2].0,
        duration[2].1,
        duration[2].2,
        duration[2].3,
        source.len()
    );
}

fn main() {
    let argument = Argument::parse();
    let left = argument.left;
    let right = argument.right;
    assert!(right > 0, "use a nonzero divisor for this benchmark");
    let sum = left
        .checked_add(right)
        .expect("reference addition overflow");
    let product = left
        .checked_mul(right)
        .expect("reference multiplication overflow");
    for radix in [2, 3] {
        let mut width = 1;
        let mut value = left.max(right);
        while value >= radix as u64 {
            value /= radix as u64;
            width += 1;
        }
        println!("radix={radix} width={width} left={left} right={right}");
        measure(
            radix,
            "add",
            circuit::add(radix, width, left, right).unwrap(),
            encoding::unsigned(radix, width + 1, sum).unwrap(),
        );
        measure(
            radix,
            "subtract",
            circuit::subtract(radix, width, left, right).unwrap(),
            encoding::difference(radix, width, left as i128 - right as i128).unwrap(),
        );
        measure(
            radix,
            "multiply",
            circuit::multiply(radix, width, left, right, circuit::Layout::Column).unwrap(),
            encoding::unsigned(radix, width * 2, product).unwrap(),
        );
        measure(
            radix,
            "balanced",
            circuit::multiply(radix, width, left, right, circuit::Layout::Balanced).unwrap(),
            encoding::unsigned(radix, width * 2, product).unwrap(),
        );
        measure(
            radix,
            "divide",
            circuit::divide(radix, width, left, right).unwrap(),
            encoding::quotient(radix, width, left / right, left % right, false).unwrap(),
        );
    }
}
