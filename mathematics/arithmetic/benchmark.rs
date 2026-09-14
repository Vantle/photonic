use arithmetic::{circuit, encoding};
use photonic::lowering::parse;
use photonic::path::Search;
use photonic::prism::Outcome;
use photonic::runtime::Limit;
use std::time::Instant;

fn measure(radix: u8, operation: &str, source: String, target: String) {
    let mut duration = Vec::new();
    for _ in 0..5 {
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
    for (radix, width) in [(2, 11), (3, 7)] {
        measure(
            radix,
            "add",
            circuit::add(radix, width, 1500, 123).unwrap(),
            encoding::unsigned(radix, width + 1, 1623).unwrap(),
        );
        measure(
            radix,
            "subtract",
            circuit::subtract(radix, width, 1500, 123).unwrap(),
            encoding::difference(radix, width, 1377).unwrap(),
        );
        measure(
            radix,
            "multiply",
            circuit::multiply(radix, width, 1500, 123, circuit::Layout::Column).unwrap(),
            encoding::unsigned(radix, width * 2, 184500).unwrap(),
        );
        measure(
            radix,
            "balanced",
            circuit::multiply(radix, width, 1500, 123, circuit::Layout::Balanced).unwrap(),
            encoding::unsigned(radix, width * 2, 184500).unwrap(),
        );
        measure(
            radix,
            "divide",
            circuit::divide(radix, width, 1500, 123).unwrap(),
            encoding::quotient(radix, width, 12, 24, false).unwrap(),
        );
    }
}
