use super::execute;
use arithmetic::{circuit, encoding};
use photonic::lowering::parse;
use photonic::obsidian::Outcome;

#[test]
fn operation() {
    for left in 0..9 {
        for right in 0..9 {
            let source = circuit::add(3, 2, left, right).unwrap();
            assert_eq!(
                execute(&source, &encoding::unsigned(3, 3, left + right).unwrap()),
                Outcome::Reached
            );
            assert_eq!(
                execute(
                    &source,
                    &encoding::unsigned(3, 3, left + right + 1).unwrap()
                ),
                Outcome::Unknown
            );
            for layout in [circuit::Layout::Column, circuit::Layout::Balanced] {
                let source = circuit::multiply(3, 2, left, right, layout).unwrap();
                assert_eq!(
                    execute(&source, &encoding::unsigned(3, 4, left * right).unwrap()),
                    Outcome::Reached,
                    "{left} * {right}"
                );
                assert_eq!(
                    execute(
                        &source,
                        &encoding::unsigned(3, 4, left * right + 1).unwrap()
                    ),
                    Outcome::Unknown
                );
            }
            let source = circuit::subtract(3, 2, left, right).unwrap();
            let expected = left as i128 - right as i128;
            assert_eq!(
                execute(&source, &encoding::difference(3, 2, expected).unwrap()),
                Outcome::Reached
            );
            assert_eq!(
                execute(
                    &source,
                    &encoding::difference(3, 2, if expected == 0 { 1 } else { 0 }).unwrap()
                ),
                Outcome::Unknown
            );
            let source = circuit::divide(3, 2, left, right).unwrap();
            let (quotient, remainder) = u64::checked_div(left, right)
                .map_or((0, left), |quotient| (quotient, left % right));
            assert_eq!(
                execute(
                    &source,
                    &encoding::quotient(3, 2, quotient, remainder, right == 0).unwrap()
                ),
                Outcome::Reached,
                "{left} / {right}"
            );
            assert_eq!(
                execute(
                    &source,
                    &encoding::quotient(3, 2, (quotient + 1) % 9, remainder, right == 0).unwrap()
                ),
                Outcome::Unknown
            );
            assert_eq!(
                execute(
                    &source,
                    &encoding::quotient(3, 2, quotient, (remainder + 1) % 9, right == 0).unwrap()
                ),
                Outcome::Unknown
            );
            assert_eq!(
                execute(
                    &source,
                    &encoding::quotient(3, 2, quotient, remainder, right != 0).unwrap()
                ),
                Outcome::Unknown
            );
        }
    }
}

#[test]
fn boundary() {
    for (left, right) in [
        (1500, 123),
        (123, 1500),
        (2186, 2186),
        (2186, 1),
        (2186, 0),
        (0, 2186),
    ] {
        assert_eq!(
            execute(
                &circuit::add(3, 7, left, right).unwrap(),
                &encoding::unsigned(3, 8, left + right).unwrap()
            ),
            Outcome::Reached
        );
        assert_eq!(
            execute(
                &circuit::multiply(3, 7, left, right, circuit::Layout::Column).unwrap(),
                &encoding::unsigned(3, 14, left * right).unwrap()
            ),
            Outcome::Reached
        );
        assert_eq!(
            execute(
                &circuit::subtract(3, 7, left, right).unwrap(),
                &encoding::difference(3, 7, left as i128 - right as i128).unwrap()
            ),
            Outcome::Reached
        );
        let (quotient, remainder) =
            u64::checked_div(left, right).map_or((0, left), |quotient| (quotient, left % right));
        assert_eq!(
            execute(
                &circuit::divide(3, 7, left, right).unwrap(),
                &encoding::quotient(3, 7, quotient, remainder, right == 0).unwrap()
            ),
            Outcome::Reached
        );
    }
    let largest = 3u64.pow(20) - 1;
    assert_eq!(
        execute(
            &circuit::add(3, 20, largest, 1).unwrap(),
            &encoding::unsigned(3, 21, largest + 1).unwrap()
        ),
        Outcome::Reached
    );
    assert_eq!(
        execute(
            &circuit::subtract(3, 20, 0, largest).unwrap(),
            &encoding::difference(3, 20, -(largest as i128)).unwrap()
        ),
        Outcome::Reached
    );
}

#[test]
fn structure() {
    for program in [circuit::add, circuit::subtract, circuit::divide] {
        assert_eq!(
            parse(&program(3, 2, 0, 0).unwrap()).unwrap().rule,
            parse(&program(3, 2, 8, 5).unwrap()).unwrap().rule
        );
    }
    for layout in [circuit::Layout::Column, circuit::Layout::Balanced] {
        assert_eq!(
            parse(&circuit::multiply(3, 2, 0, 0, layout).unwrap())
                .unwrap()
                .rule,
            parse(&circuit::multiply(3, 2, 8, 5, layout).unwrap())
                .unwrap()
                .rule
        );
    }
}

#[test]
fn exhaustive() {
    for left in 0..3 {
        for right in 0..3 {
            for expected in 0..6 {
                for (source, value) in [
                    (circuit::add(3, 1, left, right).unwrap(), left + right),
                    (
                        circuit::multiply(3, 1, left, right, circuit::Layout::Column).unwrap(),
                        left * right,
                    ),
                ] {
                    let mut search = photonic::obsidian::Search::new(
                        parse(&source).unwrap(),
                        parse(&encoding::unsigned(3, 2, expected).unwrap()).unwrap(),
                    )
                    .unwrap();
                    search.run(100_000, None);
                    let report = search.report();
                    assert!(report.execution.closed);
                    assert_eq!(
                        report.outcome,
                        if value == expected {
                            Outcome::Reached
                        } else {
                            Outcome::Unreachable
                        }
                    );
                }
            }
        }
    }
}

#[test]
fn digit() {
    let rule = concat!(
        include_str!("../../../library/application.particle"),
        "\n",
        include_str!("../../../library/ternary.particle")
    );
    let name = ["0", "1", "2"];
    for left in 0..3 {
        for right in 0..3 {
            for carry in 0..3 {
                let value = left + right + carry;
                assert_eq!(
                    execute(
                        &format!(
                            "Invoke.Sum.{}.{}.{}
{rule}",
                            name[left], name[right], name[carry]
                        ),
                        &format!(
                            "([Digit] {}).([Carry] {})",
                            name[value % 3],
                            name[value / 3]
                        )
                    ),
                    Outcome::Reached
                );
            }
            let value = left * right;
            assert_eq!(
                execute(
                    &format!(
                        "Invoke.Multiply.{}.{}
{rule}",
                        name[left], name[right]
                    ),
                    &format!(
                        "([Digit] {}).([Carry] {})",
                        name[value % 3],
                        name[value / 3]
                    )
                ),
                Outcome::Reached
            );
            for borrow in 0..2 {
                let value = left as i32 - right as i32 - borrow as i32;
                assert_eq!(
                    execute(
                        &format!(
                            "Invoke.Subtract.([Left] {}).([Right] {}).([Borrow] {})
{rule}",
                            name[left], name[right], name[borrow]
                        ),
                        &format!(
                            "([Digit] {}).([Borrow] {})",
                            name[value.rem_euclid(3) as usize],
                            name[usize::from(value < 0)]
                        )
                    ),
                    Outcome::Reached
                );
                assert_eq!(
                    execute(
                        &format!(
                            "Invoke.Select.([Left] {}).([Right] {}).([Choice] {})
{rule}",
                            name[left], name[right], name[borrow]
                        ),
                        &format!("([Digit] {})", name[if borrow == 0 { left } else { right }])
                    ),
                    Outcome::Reached
                );
            }
        }
    }
    assert_eq!(
        execute(
            &circuit::subtract(3, 1, 2, 2).unwrap(),
            "([Digit.0] 0).([Negative] 1)"
        ),
        Outcome::Unknown
    );
}

#[test]
fn notation() {
    let numeral = include_str!("../../ternary/numeral.particle").trim();
    let carry = include_str!("../../ternary/carry.particle");
    let source = format!("Add.{numeral}, Add.3^0 [Add, Add] () {carry}");
    for (target, expected) in [
        ("3^3.3^2.3^2.3^1", Outcome::Reached),
        (numeral, Outcome::Unreachable),
    ] {
        let mut search =
            photonic::obsidian::Search::new(parse(&source).unwrap(), parse(target).unwrap())
                .unwrap();
        search.run(100_000, None);
        let report = search.report();
        assert!(report.execution.closed);
        assert_eq!(report.outcome, expected);
    }
    assert_eq!(
        execute(
            include_str!("../../ternary/word.particle"),
            "Number.([Base] 3).([3] 1).([2] 2).([1] 0).([0] 2)"
        ),
        Outcome::Reached
    );
}

#[test]
fn example() {
    for (source, target, expected) in [
        (
            include_str!("../fixture/ternary/add.wave"),
            include_str!("../fixture/ternary/add.particle"),
            encoding::unsigned(3, 8, 1623).unwrap(),
        ),
        (
            include_str!("../fixture/ternary/multiply.wave"),
            include_str!("../fixture/ternary/multiply.particle"),
            encoding::unsigned(3, 14, 184_500).unwrap(),
        ),
        (
            include_str!("../fixture/ternary/subtract.wave"),
            include_str!("../fixture/ternary/subtract.particle"),
            encoding::difference(3, 7, 1377).unwrap(),
        ),
        (
            include_str!("../fixture/ternary/divide.wave"),
            include_str!("../fixture/ternary/divide.particle"),
            encoding::quotient(3, 7, 12, 24, false).unwrap(),
        ),
    ] {
        assert_eq!(target, expected);
        assert_eq!(execute(source, target), Outcome::Reached);
    }
}
