use crate::answer;
use crate::catalog::source;

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
                &format!("Invoke.Carry.Combine.([Left] {first}).([Right] {second})"),
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
                &format!("Invoke.Carry.Equal.{first}.{second}"),
                if first == second { "True" } else { "False" },
                ["True", "False"],
                &library(),
            );
        }
    }
}

#[test]
fn evaluate() {
    for state in STATE {
        for input in ["0", "1"] {
            answer(
                &format!("Invoke.Carry.Evaluate.{state}.{input}"),
                match state {
                    "Kill" => "0",
                    "Generate" => "1",
                    _ => input,
                },
                ["0", "1"],
                &library(),
            );
        }
    }
}
