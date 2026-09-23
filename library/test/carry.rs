use crate::catalog::source;
use crate::check;
use photonic::prism::Outcome;

const STATE: [&str; 3] = ["Kill", "Propagate", "Generate"];

fn library() -> [&'static str; 3] {
    [
        source("function", "invoke"),
        source("carry", "combine"),
        source("carry", "evaluate"),
    ]
}

#[test]
fn combine() {
    for (left, first) in STATE.iter().enumerate() {
        for (right, second) in STATE.iter().enumerate() {
            let source = format!("Invoke.Carry.Combine.([Left] {first}).([Right] {second})");
            let expected = if right == 1 { left } else { right };
            for (target, &value) in STATE.iter().enumerate() {
                check(
                    &source,
                    value,
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

#[test]
fn evaluate() {
    for state in STATE {
        for input in ["0", "1"] {
            let source = format!("Invoke.Carry.Evaluate.{state}.{input}");
            let expected = match state {
                "Kill" => "0",
                "Generate" => "1",
                _ => input,
            };
            for target in ["0", "1"] {
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
