use crate::prism::Outcome;
use crate::runtime::{Limit, Runtime};
use frontend::lowering::parse;

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
    let program = parse(source).unwrap();
    let goal = crate::test::target(&program, target);
    let mut runtime = Runtime::new(&program);
    runtime.run(
        1_000_000,
        Limit {
            configuration: 2000,
            occurrence: 80,
            ..Limit::default()
        },
    );
    let verdict = runtime.verdict(&goal);
    assert_eq!(verdict.outcome, Outcome::Reached, "{source} => {target}");
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
    let program = parse(source).unwrap();
    let goal = crate::test::target(&program, &numeral(1622));
    let mut runtime = Runtime::new(&program);
    runtime.run(
        1_000_000,
        Limit {
            configuration: 2000,
            occurrence: 80,
            ..Limit::default()
        },
    );
    let verdict = runtime.verdict(&goal);
    let report = runtime.snapshot();
    assert!(report.closed);
    assert_eq!(verdict.outcome, Outcome::Unreachable);
    for state in report.state {
        let total: u64 = state
            .world
            .iter()
            .flat_map(|world| &world.particle)
            .filter_map(|token| crate::test::atom(token).and_then(|atom| atom.strip_prefix("2^")))
            .map(|bit| 1u64 << bit.parse::<u32>().unwrap())
            .sum();
        assert_eq!(total, 1623);
    }
}
