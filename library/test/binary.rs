use crate::answer;
use crate::catalog::source;

fn library() -> [&'static str; 3] {
    [
        source("function", "invoke"),
        source("binary", "sum"),
        source("binary", "multiply"),
    ]
}

fn result(value: u8) -> String {
    format!("([Digit] {}).([Carry] {})", value % 2, value / 2)
}

#[test]
fn sum() {
    for mask in 0..8u8 {
        let bit = (0..3).map(|shift| (mask >> shift) & 1).collect::<Vec<_>>();
        let count = bit.iter().sum::<u8>();
        answer(
            &format!(
                "Invoke.Binary.Sum.{}",
                bit.iter().map(u8::to_string).collect::<Vec<_>>().join(".")
            ),
            &result(count),
            (0..4).map(result),
            &library(),
        );
    }
}

#[test]
fn product() {
    for left in 0..2u8 {
        for right in 0..2u8 {
            answer(
                &format!("Invoke.Binary.Multiply.{left}.{right}"),
                &(left * right).to_string(),
                ["0", "1"],
                &library(),
            );
        }
    }
}
