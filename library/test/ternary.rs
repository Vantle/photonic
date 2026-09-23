use crate::catalog::source;
use crate::check;
use photonic::prism::Outcome;

const DIGIT: [&str; 3] = ["0", "1", "2"];

fn library() -> [&'static str; 8] {
    [
        source("function", "invoke"),
        source("ternary", "add"),
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
fn sum() {
    for (left, first) in DIGIT.iter().enumerate() {
        for (right, second) in DIGIT.iter().enumerate() {
            for (carry, third) in DIGIT.iter().enumerate() {
                let source = format!("Invoke.Ternary.Sum.{first}.{second}.{third}");
                for value in 0..9 {
                    check(
                        &source,
                        &result(value),
                        &library(),
                        if value == left + right + carry {
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
fn subtract() {
    for (left, first) in DIGIT.iter().enumerate() {
        for (right, second) in DIGIT.iter().enumerate() {
            for (borrow, third) in DIGIT[..2].iter().enumerate() {
                let value = left as isize - right as isize - borrow as isize;
                let source = format!(
                    "Invoke.Ternary.Subtract.([Left] {first}).([Right] {second}).([Borrow] {third})"
                );
                for (digit, name) in DIGIT.iter().enumerate() {
                    for (negative, flag) in DIGIT[..2].iter().enumerate() {
                        check(
                            &source,
                            &format!("([Digit] {name}).([Borrow] {flag})"),
                            &library(),
                            if digit as isize == value.rem_euclid(3)
                                && (negative == 1) == (value < 0)
                            {
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
}

#[test]
fn select() {
    for (left, first) in DIGIT.iter().enumerate() {
        for (right, second) in DIGIT.iter().enumerate() {
            for (choice, third) in DIGIT[..2].iter().enumerate() {
                let source = format!(
                    "Invoke.Ternary.Select.([Left] {first}).([Right] {second}).([Choice] {third})"
                );
                let expected = if choice == 0 { left } else { right };
                for (digit, name) in DIGIT.iter().enumerate() {
                    check(
                        &source,
                        &format!("([Digit] {name})"),
                        &library(),
                        if digit == expected {
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
