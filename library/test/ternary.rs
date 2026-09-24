use crate::catalog::source;
use crate::{answer, check};
use photonic::prism::Outcome;
use std::cmp::Ordering;

const DIGIT: [&str; 3] = ["0", "1", "2"];

fn library() -> [&'static str; 9] {
    [
        source("function", "invoke"),
        source("ternary", "add"),
        source("ternary", "equal"),
        source("ternary", "multiply"),
        source("ternary", "successor"),
        source("ternary", "compare"),
        source("ternary", "sum"),
        source("ternary", "subtract"),
        source("ternary", "select"),
    ]
}

fn result(value: usize) -> String {
    format!(
        "([Digit] {}).([Carry] {})",
        DIGIT[value % 3],
        DIGIT[value / 3]
    )
}

fn difference(digit: &str, borrow: &str) -> String {
    format!("([Digit] {digit}).([Borrow] {borrow})")
}

#[test]
fn product() {
    for (left, first) in DIGIT.iter().enumerate() {
        for (right, second) in DIGIT.iter().enumerate() {
            for (operation, expected) in [("Add", left + right), ("Multiply", left * right)] {
                answer(
                    &format!("Invoke.Ternary.{operation}.{first}.{second}"),
                    &result(expected),
                    (0..9).map(result),
                    &library(),
                );
            }
        }
    }
}

#[test]
fn sum() {
    for (left, first) in DIGIT.iter().enumerate() {
        for (right, second) in DIGIT.iter().enumerate() {
            for (carry, third) in DIGIT.iter().enumerate() {
                answer(
                    &format!("Invoke.Ternary.Sum.{first}.{second}.{third}"),
                    &result(left + right + carry),
                    (0..9).map(result),
                    &library(),
                );
            }
        }
    }
}

#[test]
fn subtract() {
    for (left, first) in DIGIT.iter().enumerate() {
        for (right, second) in DIGIT.iter().enumerate() {
            for (borrow, third) in DIGIT[..2].iter().enumerate() {
                let value = left + 3 - right - borrow;
                answer(
                    &format!(
                        "Invoke.Ternary.Subtract.([Left] {first}).([Right] {second}).([Borrow] {third})"
                    ),
                    &difference(DIGIT[value % 3], DIGIT[usize::from(value < 3)]),
                    DIGIT.iter().flat_map(|digit| {
                        DIGIT[..2]
                            .iter()
                            .map(move |borrow| difference(digit, borrow))
                    }),
                    &library(),
                );
            }
        }
    }
}

#[test]
fn select() {
    for first in DIGIT {
        for second in DIGIT {
            for (choice, chosen) in [("0", first), ("1", second)] {
                answer(
                    &format!(
                        "Invoke.Ternary.Select.([Left] {first}).([Right] {second}).([Choice] {choice})"
                    ),
                    &format!("([Digit] {chosen})"),
                    DIGIT.map(|digit| format!("([Digit] {digit})")),
                    &library(),
                );
            }
        }
    }
}

#[test]
fn successor() {
    for (value, digit) in DIGIT.iter().enumerate() {
        check(
            &format!("Invoke.Ternary.Successor.{digit}"),
            &result(value + 1),
            &library(),
            Outcome::Reached,
        );
    }
}

#[test]
fn equal() {
    for first in DIGIT {
        for second in DIGIT {
            answer(
                &format!("Invoke.Ternary.Equal.{first}.{second}"),
                if first == second { "True" } else { "False" },
                ["True", "False"],
                &library(),
            );
        }
    }
}

#[test]
fn compare() {
    for (left, first) in DIGIT.iter().enumerate() {
        for (right, second) in DIGIT.iter().enumerate() {
            answer(
                &format!("Invoke.Ternary.Compare.([Left] {first}).([Right] {second})"),
                match left.cmp(&right) {
                    Ordering::Less => "Less",
                    Ordering::Equal => "Equal",
                    Ordering::Greater => "Greater",
                },
                ["Less", "Equal", "Greater"],
                &library(),
            );
        }
    }
}
