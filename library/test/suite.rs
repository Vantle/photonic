mod boolean;
mod carry;
mod catalog;
mod collection;
mod composition;
mod field;
mod function;
mod isolation;
mod selection;
mod ternary;

use photonic::lowering::parse;
use photonic::prism::{Outcome, Search};
use photonic::runtime::Limit;

pub fn program(source: &str, library: &[&str]) -> photonic::source::Program {
    parse(
        &std::iter::once(source)
            .chain(library.iter().copied())
            .collect::<Vec<_>>()
            .join("\n"),
    )
    .unwrap()
}

pub fn check(source: &str, target: &str, library: &[&str], expected: Outcome) {
    let program = program(source, library);
    let cell = 128 + program.rule.len();
    let mut search = {
        let target = photonic::source::Program {
            rule: program.rule.clone(),
            ..parse(target).unwrap()
        };
        Search::new(program, target)
    };
    search.run(
        2_000_000,
        Some(Limit {
            state: 20000,
            record: 2_000_000,
            cell,
            world: 32,
            frame: 32,
        }),
    );
    let report = search.report();
    assert!(
        report.execution.closed,
        "{source}: {} states, {} work",
        report.execution.state.len(),
        report.execution.work
    );
    assert_eq!(report.outcome, expected, "{source} => {target}");
}

pub fn witness(source: &str, target: &str, library: &[&str]) -> photonic::path::Report {
    let program = program(source, library);
    let cell = 256 + program.rule.len();
    let mut search = {
        let target = photonic::source::Program {
            rule: program.rule.clone(),
            ..parse(target).unwrap()
        };
        photonic::path::Search::new(program, target)
    };
    search.run(
        2_000_000,
        Limit {
            state: 4096,
            record: 2_000_000,
            cell,
            world: 64,
            frame: 64,
        },
    );
    let report = search.report();
    assert_eq!(
        report.outcome,
        Outcome::Reached,
        "{source} => {target}: {} work; {:?}",
        report.work,
        report.state.last()
    );
    report
}
