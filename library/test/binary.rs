use crate::catalog::source;
use crate::check;
use photonic::prism::Outcome;

fn library() -> [&'static str; 3] {
    [
        source("function", "invoke"),
        source("binary", "sum"),
        source("binary", "multiply"),
    ]
}

#[test]
fn sum() {
    for mask in 0..8u8 {
        let bit = (0..3).map(|shift| (mask >> shift) & 1).collect::<Vec<_>>();
        let count = bit.iter().sum::<u8>();
        let source = format!(
            "Invoke.Binary.Sum.{}",
            bit.iter().map(u8::to_string).collect::<Vec<_>>().join(".")
        );
        for digit in 0..2 {
            for carry in 0..2 {
                check(
                    &source,
                    &format!("([Digit] {digit}).([Carry] {carry})"),
                    &library(),
                    if digit == count % 2 && carry == count / 2 {
                        Outcome::Reached
                    } else {
                        Outcome::Unreachable
                    },
                );
            }
        }
    }
}

#[test]
fn product() {
    for left in 0..2u8 {
        for right in 0..2u8 {
            for value in 0..2u8 {
                check(
                    &format!("Invoke.Binary.Multiply.{left}.{right}"),
                    &value.to_string(),
                    &library(),
                    if value == left * right {
                        Outcome::Reached
                    } else {
                        Outcome::Unreachable
                    },
                );
            }
        }
    }
}
