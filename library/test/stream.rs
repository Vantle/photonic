use frontend::lowering::parse;
use photonic::path::{Report, Search};
use photonic::prism::Outcome;
use photonic::runtime::Limit;

const RULE: &str = include_str!("../stream/successor.particle");

fn digit(mut value: u128) -> Vec<usize> {
    let mut result = Vec::new();
    while value != 0 {
        result.push((value % 3) as usize);
        value /= 3;
    }
    result
}

fn source(digit: &[usize]) -> String {
    let mut body = "End".to_owned();
    for &value in digit.iter().rev() {
        let name = ["0", "1", "2"][value];
        body = format!("({name}, [Next] {body})");
    }
    format!(
        "Tape.Function.Stream.Successor, [Tape] {body},
{RULE}"
    )
}

fn execute(source: &str) -> Report {
    let mut search = {
        let program = parse(source).unwrap();
        let mut target = parse("Return.Stream.Successor").unwrap();
        target.preserve(&program);
        Search::new(program, Some(target))
    };
    search.run(
        100_000,
        Limit {
            configuration: 1024,
            record: 1_000_000,
            occurrence: 4096,
            scope: 128,
            coherence: 4,
        },
    );
    let report = search.report();
    assert_eq!(
        report.outcome,
        Outcome::Reached,
        "work={}, events={}, last={:?}",
        report.work,
        report.event.len(),
        report
            .state
            .last()
            .map(|node| (node.frame.len(), &node.world))
    );
    report
}

fn written(report: &Report) -> Vec<usize> {
    report
        .event
        .iter()
        .filter_map(|event| {
            ["0", "1", "2"].iter().position(|name| {
                report.definition[event.rule].name == format!("[[Write] {name}] Next")
            })
        })
        .collect()
}

#[test]
fn successor() {
    for value in 0..243 {
        let input = digit(value);
        let report = execute(&source(&input));
        assert_eq!(written(&report), digit(value + 1), "{value}");
        assert!(report.event.len() <= 3 * input.len() + 4);
    }
}

#[test]
fn carry() {
    for width in [1, 2, 3, 10, 20, 40] {
        let report = execute(&source(&vec![2; width]));
        assert_eq!(
            written(&report),
            std::iter::repeat_n(0, width).chain([1]).collect::<Vec<_>>()
        );
        assert_eq!(report.event.len(), 3 * width + 4);
    }
    let value = u64::MAX as u128;
    assert_eq!(written(&execute(&source(&digit(value)))), digit(value + 1));
}

#[test]
fn suffix() {
    for (input, expected) in [
        (vec![0, 2, 1, 0, 2], vec![1, 2, 1, 0, 2]),
        (vec![1, 0, 2, 1, 2], vec![2, 0, 2, 1, 2]),
        (vec![2, 2, 0, 1, 2], vec![0, 0, 1, 1, 2]),
        (vec![0, 0, 0], vec![1, 0, 0]),
    ] {
        assert_eq!(written(&execute(&source(&input))), expected);
    }
}

#[test]
fn library() {
    let rule = parse(RULE).unwrap().rule;
    assert_eq!(rule.len(), 12);
    for value in [0, 1, 20, 1500, u64::MAX as u128] {
        let program = parse(&source(&digit(value))).unwrap();
        assert_eq!(&program.rule[1..], rule);
    }
    let report = execute(&format!(
        "{},
{RULE}",
        include_str!("../../program/ternary/stream.wave")
    ));
    assert_eq!(written(&report), [0, 0, 2]);
    assert_eq!(report.event.len(), 11);
    assert!(report.work < 1000);
}

#[test]
fn tape() {
    for digit in [
        vec![],
        vec![true],
        vec![false, true],
        vec![true, false, true],
    ] {
        let mut body = "End".to_owned();
        for value in digit.iter().rev() {
            body = format!("({}, [Next] {body})", if *value { "1" } else { "0" });
        }
        let source = format!("Read, [Read] {body}, [0] Next, [1] Next");
        let mut search = {
            let program = parse(&source).unwrap();
            let mut target = parse("End").unwrap();
            target.preserve(&program);
            Search::new(program, Some(target))
        };
        search.run(
            100_000,
            Limit {
                scope: 32,
                occurrence: 100,
                ..Limit::default()
            },
        );
        let report = search.report();
        assert_eq!(report.outcome, Outcome::Reached);
        let visited = report
            .event
            .iter()
            .filter_map(|event| match report.definition[event.rule].name.as_str() {
                "[0] Next" => Some(false),
                "[1] Next" => Some(true),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(visited, digit);
    }
}
