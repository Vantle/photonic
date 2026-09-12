use super::execute;
use arithmetic::{circuit, encoding, gate};
use photonic::lowering::parse;
use photonic::obsidian::Outcome;

#[test]
fn operation() {
    for left in 0..9 {
        for right in 0..9 {
            let source = circuit::add(3, 2, left, right);
            assert_eq!(
                execute(&source, &encoding::unsigned(3, 3, left + right)),
                Outcome::Reached
            );
            assert_eq!(
                execute(&source, &encoding::unsigned(3, 3, left + right + 1)),
                Outcome::Unknown
            );
            for layout in [circuit::Layout::Column, circuit::Layout::Balanced] {
                let source = circuit::multiply(3, 2, left, right, layout);
                assert_eq!(
                    execute(&source, &encoding::unsigned(3, 4, left * right)),
                    Outcome::Reached,
                    "{left} * {right}"
                );
                assert_eq!(
                    execute(&source, &encoding::unsigned(3, 4, left * right + 1)),
                    Outcome::Unknown
                );
            }
            let source = circuit::subtract(3, 2, left, right);
            let expected = left as i128 - right as i128;
            assert_eq!(
                execute(&source, &encoding::difference(3, 2, expected)),
                Outcome::Reached
            );
            assert_eq!(
                execute(
                    &source,
                    &encoding::difference(3, 2, if expected == 0 { 1 } else { 0 })
                ),
                Outcome::Unknown
            );
            let source = circuit::divide(3, 2, left, right);
            let (quotient, remainder) = if right == 0 {
                (0, left)
            } else {
                (left / right, left % right)
            };
            assert_eq!(
                execute(
                    &source,
                    &encoding::quotient(3, 2, quotient, remainder, right == 0)
                ),
                Outcome::Reached,
                "{left} / {right}"
            );
            assert_eq!(
                execute(
                    &source,
                    &encoding::quotient(3, 2, (quotient + 1) % 9, remainder, right == 0)
                ),
                Outcome::Unknown
            );
            assert_eq!(
                execute(
                    &source,
                    &encoding::quotient(3, 2, quotient, (remainder + 1) % 9, right == 0)
                ),
                Outcome::Unknown
            );
            assert_eq!(
                execute(
                    &source,
                    &encoding::quotient(3, 2, quotient, remainder, right != 0)
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
                &circuit::add(3, 7, left, right),
                &encoding::unsigned(3, 8, left + right)
            ),
            Outcome::Reached
        );
        assert_eq!(
            execute(
                &circuit::multiply(3, 7, left, right, circuit::Layout::Column),
                &encoding::unsigned(3, 14, left * right)
            ),
            Outcome::Reached
        );
        assert_eq!(
            execute(
                &circuit::subtract(3, 7, left, right),
                &encoding::difference(3, 7, left as i128 - right as i128)
            ),
            Outcome::Reached
        );
        let (quotient, remainder) = if right == 0 {
            (0, left)
        } else {
            (left / right, left % right)
        };
        assert_eq!(
            execute(
                &circuit::divide(3, 7, left, right),
                &encoding::quotient(3, 7, quotient, remainder, right == 0)
            ),
            Outcome::Reached
        );
    }
    let largest = 3u64.pow(20) - 1;
    assert_eq!(
        execute(
            &circuit::add(3, 20, largest, 1),
            &encoding::unsigned(3, 21, largest + 1)
        ),
        Outcome::Reached
    );
    assert_eq!(
        execute(
            &circuit::subtract(3, 20, 0, largest),
            &encoding::difference(3, 20, -(largest as i128))
        ),
        Outcome::Reached
    );
}

#[test]
fn structure() {
    for program in [circuit::add, circuit::subtract, circuit::divide] {
        assert_eq!(
            parse(&program(3, 2, 0, 0)).unwrap().rule,
            parse(&program(3, 2, 8, 5)).unwrap().rule
        );
    }
    for layout in [circuit::Layout::Column, circuit::Layout::Balanced] {
        assert_eq!(
            parse(&circuit::multiply(3, 2, 0, 0, layout)).unwrap().rule,
            parse(&circuit::multiply(3, 2, 8, 5, layout)).unwrap().rule
        );
    }
    for left in 0..3 {
        for right in 0..3 {
            for borrow in 0..2 {
                let output = gate::Kind::Difference.evaluate(3, &[left, right, borrow]);
                assert_eq!(
                    left as i32 - right as i32 - borrow as i32,
                    output[0] as i32 - 3 * output[1] as i32
                );
            }
        }
    }
}

#[test]
fn exhaustive() {
    for left in 0..3 {
        for right in 0..3 {
            for expected in 0..6 {
                for (source, value) in [
                    (circuit::add(3, 1, left, right), left + right),
                    (
                        circuit::multiply(3, 1, left, right, circuit::Layout::Column),
                        left * right,
                    ),
                ] {
                    let mut search = photonic::obsidian::Search::new(
                        parse(&source).unwrap(),
                        parse(&encoding::unsigned(3, 2, expected)).unwrap(),
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
    let rule = include_str!("../ternary/digit.particle");
    let name = ["Zero", "One", "Two"];
    for left in 0..3 {
        for right in 0..3 {
            for carry in 0..3 {
                let value = left + right + carry;
                assert_eq!(
                    execute(
                        &format!("Add.{}.{}.{} {rule}", name[left], name[right], name[carry]),
                        &format!("Trit{}.Carry{}", name[value % 3], name[value / 3])
                    ),
                    Outcome::Reached
                );
            }
            let value = left * right;
            assert_eq!(
                execute(
                    &format!("Multiply.{}.{} {rule}", name[left], name[right]),
                    &format!("Trit{}.Carry{}", name[value % 3], name[value / 3])
                ),
                Outcome::Reached
            );
            for borrow in 0..2 {
                let value = left as i32 - right as i32 - borrow as i32;
                assert_eq!(
                    execute(
                        &format!(
                            "Subtract.Left{}.Right{}.Borrow{} {rule}",
                            name[left], name[right], name[borrow]
                        ),
                        &format!(
                            "Trit{}.Borrow{}",
                            name[value.rem_euclid(3) as usize],
                            name[usize::from(value < 0)]
                        )
                    ),
                    Outcome::Reached
                );
                assert_eq!(
                    execute(
                        &format!(
                            "Select.Left{}.Right{}.Choice{} {rule}",
                            name[left], name[right], name[borrow]
                        ),
                        &format!("Trit{}", name[if borrow == 0 { left } else { right }])
                    ),
                    Outcome::Reached
                );
            }
        }
    }
    assert_eq!(
        execute(&circuit::subtract(3, 1, 2, 2), "Trit0Zero.NegativeOne"),
        Outcome::Unknown
    );
}
