use crate::lowering::parse;
use crate::obsidian::{Outcome, Search};
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
    let mut search = Search::new(parse(source).unwrap(), parse(target).unwrap()).unwrap();
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
    let rule = include_str!("../../mathematics/binary/carry.particle");
    for bit in 0..32 {
        check(
            &format!("2^{bit}.2^{bit} {rule}"),
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
                "Add({},{}) [Add,Add] () {rule}",
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
        include_str!("../../mathematics/binary/addition.wave"),
        include_str!("../../mathematics/binary/result.particle"),
    );
    assert_eq!(
        parse(include_str!("../../mathematics/binary/product.particle"))
            .unwrap()
            .initial[0]
            .len(),
        8
    );
    assert_eq!(
        parse(&numeral(184500)).unwrap().initial,
        parse(include_str!("../../mathematics/binary/product.particle"))
            .unwrap()
            .initial
    );
}

#[test]
fn gate() {
    let rule = concat!(
        include_str!("../../library/application.particle"),
        "\n",
        include_str!("../../library/binary.particle")
    );
    for mask in 0..8u8 {
        let source = (0..3)
            .map(|bit| if mask & (1 << bit) == 0 { "0" } else { "1" })
            .collect::<Vec<_>>()
            .join(".");
        let count = mask.count_ones();
        let sum = if count % 2 == 0 { "0" } else { "1" };
        let carry = if count < 2 { "0" } else { "1" };
        check(
            &format!("Invoke.Binary.Sum.{source}\n{rule}"),
            &format!("([Digit] {sum}).([Carry] {carry})"),
        );
    }
    let rule = concat!(
        include_str!("../../library/application.particle"),
        "\n",
        include_str!("../../library/binary.particle")
    );
    for left in ["0", "1"] {
        for right in ["0", "1"] {
            check(
                &format!("Invoke.Binary.Multiply.{left}.{right}\n{rule}"),
                if left == "1" && right == "1" {
                    "1"
                } else {
                    "0"
                },
            );
        }
    }
}

#[test]
fn conservation() {
    let source = include_str!("../../mathematics/binary/addition.wave");
    let mut search = Search::new(parse(source).unwrap(), parse(&numeral(1622)).unwrap()).unwrap();
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
    let rule = concat!(
        include_str!("../../library/application.particle"),
        "\n",
        include_str!("../../library/binary.particle")
    );
    let mut search = Search::new(
        parse(&format!("Invoke.Binary.Sum.1.1.0\n{rule}")).unwrap(),
        parse("([Digit] 1).([Carry] 1)").unwrap(),
    )
    .unwrap();
    search.run(12_000, None);
    let report = search.report();
    assert!(report.execution.closed);
    assert_eq!(report.outcome, Outcome::Unreachable);
}
