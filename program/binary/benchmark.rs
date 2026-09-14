use photonic::lowering::parse;
use photonic::prism::{Outcome, Search};
use photonic::runtime::Limit;
use std::time::Instant;

fn numeral(value: u64, radix: u32) -> String {
    if radix != 1 {
        return arithmetic::power::numeral(value, radix).unwrap();
    }
    if value == 0 {
        "()".into()
    } else {
        vec!["Unit"; value as usize].join(".")
    }
}

fn main() {
    for (left, right) in [(1500, 123), (184500, 123), (59048, 1), (65535, 1)] {
        for radix in [1, 2, 3] {
            let rule = if radix == 1 {
                String::new()
            } else {
                arithmetic::power::rule(radix, 32).unwrap()
            };
            let source = format!(
                "Add({},{}) [Add,Add] () {rule}",
                numeral(left, radix),
                numeral(right, radix)
            );
            let target = numeral(left + right, radix);
            let mut duration = Vec::new();
            let mut count = 0;
            for _ in 0..3 {
                let start = Instant::now();
                let mut search =
                    Search::new(parse(&source).unwrap(), parse(&target).unwrap()).unwrap();
                search.run(
                    1_000_000,
                    Some(Limit {
                        state: 2000,
                        record: 4_000_000,
                        cell: 200_000,
                        ..Limit::default()
                    }),
                );
                let report = search.report();
                assert!(report.execution.closed);
                assert_eq!(report.outcome, Outcome::Reached);
                duration.push(start.elapsed().as_micros());
                count = report.execution.state.len();
            }
            duration.sort();
            println!(
                "left={left} right={right} radix={radix} microseconds={} configurations={count} result_occurrences={}",
                duration[1],
                parse(&target).unwrap().initial[0].len()
            );
        }
    }
}
