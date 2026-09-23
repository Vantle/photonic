use crate::catalog::source;
use crate::check;
use photonic::prism::Outcome;

fn library() -> [&'static str; 3] {
    [
        source("function", "invoke"),
        source("field", "pack"),
        source("field", "unpack"),
    ]
}

#[test]
fn boundary() {
    for library in [
        &[][..],
        &[source("function", "invoke")][..],
        &[source("field", "pack")][..],
    ] {
        check(
            "Invoke.Field.Pack.([Position] 0).([Value] 2)",
            "([0] 2)",
            library,
            Outcome::Unreachable,
        );
    }
    for source in [
        "Invoke.Field.Pack(Position.0, Value.2)",
        "Invoke.Field.Pack.([Position] 4).([Value] 2)",
        "Invoke.Field.Pack.([Position] 0).([Value] 3)",
        "Invoke.Field.Pack.([Value] 2)",
    ] {
        check(source, "([0] 2)", &library(), Outcome::Unreachable);
    }
    check(
        "Invoke.Field.Pack.([Position] 0).([Value] 2).Extra",
        "([0] 2).Extra",
        &library(),
        Outcome::Reached,
    );
    check(
        "Invoke.Field.Unpack.([Position] 0).([1] 2)",
        "([Value] 2)",
        &library(),
        Outcome::Unreachable,
    );
}

#[test]
fn table() {
    for index in 0..4 {
        for value in 0..3 {
            for expected in 0..3 {
                let outcome = if expected == value {
                    Outcome::Reached
                } else {
                    Outcome::Unreachable
                };
                check(
                    &format!("Invoke.Field.Unpack.([Position] {index}).([{index}] {value})"),
                    &format!("([Value] {expected})"),
                    &library(),
                    outcome,
                );
                check(
                    &format!("Invoke.Field.Pack.([Position] {index}).([Value] {value})"),
                    &format!("([{index}] {expected})"),
                    &library(),
                    outcome,
                );
            }
        }
    }
}

#[test]
fn order() {
    check(
        "Number.([Base] 3).([0] 2).([1] 0).([2] 2).([3] 1)",
        "Number.([3] 1).([1] 0).([0] 2).([Base] 3).([2] 2)",
        &[],
        Outcome::Reached,
    );
    check(
        "Number.([Base] 3).([0] 2).([1] 0)",
        "Number.([Base] 3).([0] 0).([1] 2)",
        &[],
        Outcome::Unreachable,
    );
}
