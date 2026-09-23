use crate::catalog::source;
use crate::check;
use photonic::prism::Outcome;

const DIGIT: [&str; 3] = ["0", "1", "2"];

fn library() -> [&'static str; 5] {
    [
        source("function", "invoke"),
        source("ternary", "add"),
        source("ternary", "multiply"),
        source("ternary", "successor"),
        source("ternary", "compare"),
    ]
}

fn result(value: usize) -> String {
    format!(
        "([Digit] {}).([Carry] {})",
        DIGIT[value % 3],
        DIGIT[value / 3]
    )
}

#[test]
fn product() {
    for (left, first) in DIGIT.iter().enumerate() {
        for (right, second) in DIGIT.iter().enumerate() {
            for (operation, expected) in [("Add", left + right), ("Multiply", left * right)] {
                let source = format!("Invoke.Ternary.{operation}.{first}.{second}");
                for value in 0..9 {
                    check(
                        &source,
                        &result(value),
                        &library(),
                        if value == expected {
                            Outcome::Reached
                        } else {
                            Outcome::Unreachable
                        },
                    );
                }
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
fn compare() {
    for (left, first) in DIGIT.iter().enumerate() {
        for (right, second) in DIGIT.iter().enumerate() {
            let source = format!("Invoke.Ternary.Compare.([Left] {first}).([Right] {second})");
            let expected = match left.cmp(&right) {
                std::cmp::Ordering::Less => "Less",
                std::cmp::Ordering::Equal => "Equal",
                std::cmp::Ordering::Greater => "Greater",
            };
            for target in ["Less", "Equal", "Greater"] {
                check(
                    &source,
                    target,
                    &library(),
                    if target == expected {
                        Outcome::Reached
                    } else {
                        Outcome::Unreachable
                    },
                );
            }
        }
    }
}
