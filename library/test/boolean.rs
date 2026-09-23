use crate::catalog::source;
use crate::check;
use photonic::prism::Outcome;

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
    let value = ["False", "True"];
    for left in 0..2 {
        let source = format!("Invoke.Boolean.Not.{}", value[left]);
        for (index, &target) in value.iter().enumerate() {
            check(
                &source,
                target,
                &library(),
                if index == 1 - left {
                    Outcome::Reached
                } else {
                    Outcome::Unreachable
                },
            );
        }
        for right in 0..2 {
            for (operation, result) in [
                ("And", left == 1 && right == 1),
                ("Or", left == 1 || right == 1),
                ("Equal", left == right),
            ] {
                let source = format!(
                    "Invoke.Boolean.{operation}.{}.{}",
                    value[left], value[right]
                );
                for (index, &target) in value.iter().enumerate() {
                    check(
                        &source,
                        target,
                        &library(),
                        if index == usize::from(result) {
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
