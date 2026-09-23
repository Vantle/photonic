use crate::catalog::source;
use crate::{answer, check};
use photonic::prism::Outcome;

const VALUE: [&str; 2] = ["False", "True"];

fn library() -> [&'static str; 5] {
    [
        source("function", "invoke"),
        source("boolean", "not"),
        source("boolean", "and"),
        source("boolean", "or"),
        source("boolean", "equal"),
    ]
}

#[test]
fn table() {
    for left in [false, true] {
        let first = VALUE[usize::from(left)];
        answer(
            &format!("Invoke.Boolean.Not.{first}"),
            VALUE[usize::from(!left)],
            VALUE,
            &library(),
        );
        for right in [false, true] {
            let second = VALUE[usize::from(right)];
            for (operation, result) in [
                ("And", left && right),
                ("Or", left || right),
                ("Equal", left == right),
            ] {
                answer(
                    &format!("Invoke.Boolean.{operation}.{first}.{second}"),
                    VALUE[usize::from(result)],
                    VALUE,
                    &library(),
                );
            }
        }
    }
}

#[test]
fn independence() {
    let source = "Invoke.Boolean.Not.True, Invoke.Boolean.Not.False";
    for (target, expected) in [
        ("False, True", Outcome::Reached),
        ("False, False", Outcome::Unreachable),
        ("True, True", Outcome::Unreachable),
    ] {
        check(source, target, &library(), expected);
    }
}
