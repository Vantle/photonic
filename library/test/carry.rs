use crate::catalog::source;
use crate::{answer, check};
use photonic::prism::Outcome;

const STATE: [&str; 3] = ["Kill", "Propagate", "Generate"];

fn library() -> [&'static str; 4] {
    [
        source("function", "invoke"),
        source("carry", "combine"),
        source("carry", "equal"),
        source("carry", "evaluate"),
    ]
}

#[test]
fn combine() {
    for first in STATE {
        for second in STATE {
            answer(
                &format!("Invoke.Signal.Combine.([Left] {first}).([Right] {second})"),
                if second == "Propagate" { first } else { second },
                STATE,
                &library(),
            );
        }
    }
}

#[test]
fn equal() {
    for first in STATE {
        for second in STATE {
            answer(
                &format!("Invoke.Signal.Equal.{first}.{second}"),
                if first == second { "True" } else { "False" },
                ["True", "False"],
                &library(),
            );
        }
    }
}

fn evaluation(state: &str, input: &'static str) -> &'static str {
    match state {
        "Kill" => "0",
        "Generate" => "1",
        _ => input,
    }
}

#[test]
fn evaluate() {
    for state in STATE {
        for input in ["0", "1"] {
            answer(
                &format!("Invoke.Signal.Evaluate.{state}.{input}"),
                evaluation(state, input),
                ["0", "1"],
                &library(),
            );
        }
    }
}

#[test]
fn passenger() {
    for state in STATE {
        for input in ["0", "1"] {
            check(
                &format!("Invoke.Signal.Evaluate.{state}.{input}.([Digit] 1).([Carry] 0)"),
                &format!("{}.([Digit] 1).([Carry] 0)", evaluation(state, input)),
                &library(),
                Outcome::Reached,
            );
        }
    }
}
