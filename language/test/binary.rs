use crate::lowering::parse;
use crate::prism::{Outcome, Search};
use crate::runtime::Limit;

fn numeral(value: u64) -> String {
    let result = (0..64)
        .filter(|bit| value & (1 << bit) != 0)
        .map(|bit| format!("2^{bit}"))
        .collect::<Vec<_>>()
        .join(".");
    if result.is_empty() {
        "()".into()
    } else {
        result
    }
}

fn check(source: &str, target: &str) {
    let mut search = {
        let program = parse(source).unwrap();
        let target = crate::source::Program {
            rule: program.rule.clone(),
            ..parse(target).unwrap()
        };
        Search::new(program, target)
    };
    search.run(
        1_000_000,
        Some(Limit {
            state: 2000,
            cell: 80,
            ..Limit::default()
        }),
    );
    let report = search.report();
    assert_eq!(report.outcome, Outcome::Reached, "{source} => {target}");
}

#[test]
fn carry() {
    let rule = include_str!("../../program/binary/carry.particle");
    for bit in 0..32 {
        check(
            &format!("2^{bit}.2^{bit}, {rule}"),
            &format!("2^{}", bit + 1),
        );
    }
    for (left, right) in [
        (0, 0),
        (1, 1),
        (7, 9),
        (1500, 123),
        (123, 1500),
        (u16::MAX as u64, 1),
        (u32::MAX as u64, 1),
    ] {
        check(
            &format!(
                "Add.({},{}), [Add,Add] (), {rule}",
                numeral(left),
                numeral(right)
            ),
            &numeral(left + right),
        );
    }
}

#[test]
fn example() {
    check(
        include_str!("../../program/binary/addition.wave"),
        include_str!("../../program/binary/result.particle"),
    );
    assert_eq!(
        parse(include_str!("../../program/binary/product.particle"))
            .unwrap()
            .initial[0]
            .len(),
        8
    );
    assert_eq!(
        parse(&numeral(184500)).unwrap().initial,
        parse(include_str!("../../program/binary/product.particle"))
            .unwrap()
            .initial
    );
}

#[test]
fn conservation() {
    let source = include_str!("../../program/binary/addition.wave");
    let mut search = {
        let program = parse(source).unwrap();
        let target = crate::source::Program {
            rule: program.rule.clone(),
            ..parse(&numeral(1622)).unwrap()
        };
        Search::new(program, target)
    };
    search.run(
        1_000_000,
        Some(Limit {
            state: 2000,
            cell: 80,
            ..Limit::default()
        }),
    );
    let report = search.report();
    assert!(report.execution.closed);
    assert_eq!(report.outcome, Outcome::Unreachable);
    for state in report.execution.state {
        let total: u64 = state
            .world
            .iter()
            .flat_map(|world| &world.particle)
            .filter_map(|token| token.label.strip_prefix("2^"))
            .map(|bit| 1u64 << bit.parse::<u32>().unwrap())
            .sum();
        assert_eq!(total, 1623);
    }
}
