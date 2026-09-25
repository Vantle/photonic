use crate::catalog::source;
use crate::{answer, check};
use photonic::prism::Outcome;

const KEY: [&str; 4] = ["Alpha", "Beta", "Gamma", "Delta"];

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
            "().([Alpha] 2)",
            library,
            Outcome::Unreachable,
        );
    }
    for source in [
        "Invoke.Field.Pack.(Position.0, Value.2)",
        "Invoke.Field.Pack.([Position] 4).([Value] 2)",
        "Invoke.Field.Pack.([Position] 0).([Value] 3)",
        "Invoke.Field.Pack.([Value] 2)",
    ] {
        check(source, "().([Alpha] 2)", &library(), Outcome::Unreachable);
    }
    check(
        "Invoke.Field.Pack.([Position] 0).([Value] 2).Extra",
        "([Alpha] 2).Extra",
        &library(),
        Outcome::Reached,
    );
    check(
        "Invoke.Field.Unpack.([Position] 0).([Beta] 2)",
        "().([Value] 2)",
        &library(),
        Outcome::Unreachable,
    );
}

#[test]
fn table() {
    for (index, key) in KEY.iter().enumerate() {
        for value in 0..3 {
            answer(
                &format!("Invoke.Field.Unpack.([Position] {index}).([{key}] {value})"),
                &format!("().([Value] {value})"),
                (0..3).map(|candidate| format!("().([Value] {candidate})")),
                &library(),
            );
            answer(
                &format!("Invoke.Field.Pack.([Position] {index}).([Value] {value})"),
                &format!("().([{key}] {value})"),
                (0..3).map(|candidate| format!("().([{key}] {candidate})")),
                &library(),
            );
        }
    }
}

#[test]
fn passenger() {
    for (index, key) in KEY.iter().enumerate() {
        for value in 0..3 {
            for passenger in 0..3 {
                let source =
                    format!("Invoke.Field.Pack.([Position] {index}).([Value] {value}).{passenger}");
                check(
                    &source,
                    &format!("{passenger}.([{key}] {value})"),
                    &library(),
                    Outcome::Reached,
                );
                if passenger != value {
                    check(
                        &source,
                        &format!("{value}.([{key}] {value})"),
                        &library(),
                        Outcome::Unreachable,
                    );
                }
            }
        }
    }
}

#[test]
fn order() {
    check(
        "Number.([Base] 3).([Alpha] 2).([Beta] 0).([Gamma] 2).([Delta] 1)",
        "Number.([Delta] 1).([Beta] 0).([Alpha] 2).([Base] 3).([Gamma] 2)",
        &[],
        Outcome::Reached,
    );
    check(
        "Number.([Base] 3).([Alpha] 2).([Beta] 0)",
        "Number.([Base] 3).([Alpha] 0).([Beta] 2)",
        &[],
        Outcome::Unreachable,
    );
}
