mod binary;
mod boolean;
mod carry;
mod catalog;
mod collection;
mod composition;
mod expression;
mod field;
mod fixture;
mod function;
mod isolation;
mod natural;
mod selection;
mod stream;
mod ternary;
mod vector;

use frontend::lowering::parse;
use photonic::prism::Outcome;
use photonic::runtime::{Limit, Runtime};

pub fn program(source: &str, library: &[&str]) -> frontend::source::Program {
    parse(
        &std::iter::once(source)
            .chain(library.iter().copied())
            .collect::<Vec<_>>()
            .join(",\n"),
    )
    .unwrap()
}

pub fn target(program: &frontend::source::Program, text: &str) -> frontend::source::Program {
    let mut target = parse(text).unwrap();
    target.preserve(program);
    target
}

pub fn check(source: &str, target: &str, library: &[&str], expected: Outcome) {
    let program = program(source, library);
    let goal = self::target(&program, target);
    let mut runtime = Runtime::new(&program);
    runtime.run(
        2_000_000,
        Limit {
            configuration: 20000,
            record: 2_000_000,
            occurrence: 128 + program.rule.len(),
            coherence: 32,
            scope: 32,
        },
    );
    let snapshot = runtime.snapshot();
    assert!(
        snapshot.closed,
        "{source}: {} states, {} work",
        snapshot.state.len(),
        snapshot.work
    );
    assert_eq!(
        runtime.verdict(&goal).outcome,
        expected,
        "{source} => {target}"
    );
}

pub fn answer(
    source: &str,
    expected: &str,
    candidate: impl IntoIterator<Item = impl AsRef<str>>,
    library: &[&str],
) {
    let mut found = false;
    for target in candidate {
        let target = target.as_ref();
        found |= target == expected;
        check(
            source,
            target,
            library,
            if target == expected {
                Outcome::Reached
            } else {
                Outcome::Unreachable
            },
        );
    }
    assert!(found, "{source}: {expected} is not a candidate");
}

pub fn witness(source: &str, target: &str, library: &[&str]) -> photonic::path::Report {
    let program = program(source, library);
    let occurrence = 256 + program.rule.len();
    let mut search = {
        let target = self::target(&program, target);
        photonic::path::Search::new(program, Some(target))
    };
    search.run(
        2_000_000,
        Limit {
            configuration: 4096,
            record: 2_000_000,
            occurrence,
            coherence: 64,
            scope: 64,
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
