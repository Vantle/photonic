use crate::catalog::source;
use crate::{check, program, witness};
use photonic::prism::{Outcome, Search};
use photonic::runtime::Limit;

fn invoke() -> &'static str {
    source("function", "invoke")
}

fn compose() -> [&'static str; 2] {
    [invoke(), source("function", "compose")]
}

#[test]
fn identity() {
    check(
        "Invoke.Identity.Payload",
        "Payload",
        &[invoke(), source("function", "identity")],
        Outcome::Reached,
    );
}

#[test]
fn value() {
    check(
        "Invoke.True.([Function.True] Return.False)",
        "False.([Function.True] Return.False)",
        &[invoke()],
        Outcome::Reached,
    );
    check(
        "Invoke.True.([Function.True] Return.False)",
        "True.([Function.True] Return.False)",
        &[invoke()],
        Outcome::Unreachable,
    );
}

#[test]
fn invocation() {
    let definition = "([Function.Seed] Return.Flower)";
    let source = format!("Invoke.Seed.{definition}");
    let target = format!("Flower.{definition}");
    check(&source, &target, &[invoke()], Outcome::Reached);
    check(
        &source,
        &format!("Seed.{definition}"),
        &[invoke()],
        Outcome::Unreachable,
    );
    assert_eq!(witness(&source, &target, &[invoke()]).event.len(), 3);
}

#[test]
fn extension() {
    let implementation = "[Decorate.([Function] Decorate).Envelope] Return.([Result] 11)";
    check(
        "Invoke.([Function] Decorate).Envelope",
        "([Result] 11)",
        &[invoke(), implementation],
        Outcome::Reached,
    );
    check(
        "Invoke.([Function] Decorate).Envelope",
        "Envelope",
        &[invoke(), implementation],
        Outcome::Unreachable,
    );
    check(
        "Invoke.([Function] Missing).Envelope",
        "([Result] 11)",
        &[invoke()],
        Outcome::Unreachable,
    );
}

#[test]
fn composition() {
    let library = [
        invoke(),
        source("function", "compose"),
        source("function", "identity"),
        source("boolean", "not"),
    ];
    for first in ["Boolean.Not", "Identity"] {
        for second in ["Boolean.Not", "Identity"] {
            let descriptor = format!("([First] Function.{first}).([Second] Function.{second})");
            let source = format!("Invoke.Compose.{descriptor}.True");
            let negated = (first == "Boolean.Not") != (second == "Boolean.Not");
            for (value, expected) in [
                (if negated { "False" } else { "True" }, Outcome::Reached),
                (if negated { "True" } else { "False" }, Outcome::Unreachable),
            ] {
                check(
                    &source,
                    &format!("{value}.{descriptor}"),
                    &library,
                    expected,
                );
            }
        }
    }
}

#[test]
fn generic() {
    for (input, output) in [
        ("Seed", "Flower"),
        ("7.7", "8.([Branch] Leaf)"),
        ("([Record] Item)", "([Result] ([Nested] Value))"),
    ] {
        let first = format!("([First.{input}] Return.Middle)");
        let second = format!("([Second.Middle] Return.{output})");
        let source = format!("Invoke.Compose.{input}.{first}.{second}");
        check(
            &source,
            &format!("{output}.{first}.{second}"),
            &compose(),
            Outcome::Reached,
        );
        check(
            &source,
            &format!("Wrong.{first}.{second}"),
            &compose(),
            Outcome::Unreachable,
        );
    }
}

#[test]
fn conflict() {
    let first = "([First.Seed] Return.Middle).([Second.Middle] Return.Flower)";
    let second = "([First.Seed] Return.Other).([Second.Other] Return.Tree)";
    let source = format!("Left.Invoke.Compose.Seed.{first}, Right.Invoke.Compose.Seed.{second}");
    for (target, expected) in [
        (
            format!("Left.Flower.{first}, Right.Tree.{second}"),
            Outcome::Reached,
        ),
        (
            format!("Left.Tree.{first}, Right.Flower.{second}"),
            Outcome::Unreachable,
        ),
        (
            format!("Left.Flower.{first}, Right.Flower.{second}"),
            Outcome::Unreachable,
        ),
    ] {
        check(&source, &target, &compose(), expected);
    }
}

#[test]
fn incomplete() {
    let definition = "([Function.Seed] Flower)";
    check(
        &format!("Invoke.Seed.{definition}"),
        &format!("Flower.{definition}"),
        &[invoke()],
        Outcome::Unreachable,
    );
    let first = "([First.Seed] Return.Middle)";
    check(
        &format!("Invoke.Compose.Seed.{first}"),
        &format!("Middle.{first}"),
        &compose(),
        Outcome::Unreachable,
    );
    let first = "([First.Seed] Middle)";
    let second = "([Second.Middle] Return.Flower)";
    check(
        &format!("Invoke.Compose.Seed.{first}.{second}"),
        &format!("Flower.{first}.{second}"),
        &compose(),
        Outcome::Unreachable,
    );
}

#[test]
fn obsolete() {
    for request in ["Apply.Boolean.Not.True", "Call.Boolean.Not.True"] {
        check(
            request,
            "False",
            &[invoke(), source("boolean", "not")],
            Outcome::Unreachable,
        );
    }
}

fn deterministic(source: &str, target: &str, library: &[&str]) {
    let mut baseline = None;
    for worker in [1, 2, 4] {
        let executor = photonic::executor::Executor::new(worker).unwrap();
        let mut search = {
            let program = program(source, library);
            let target = crate::target(&program, target);
            Search::new(program, target)
        };
        search.parallel(&executor, 1, None);
        assert!(!search.report().execution.closed);
        search.parallel(
            &executor,
            2_000_000,
            Some(Limit {
                state: 20000,
                record: 2_000_000,
                cell: 128,
                world: 32,
                frame: 32,
            }),
        );
        let report = search.report();
        assert!(report.execution.closed);
        assert_eq!(report.outcome, Outcome::Reached);
        let actual = format!("{:?}", report.execution);
        if let Some(expected) = &baseline {
            assert_eq!(&actual, expected);
        }
        baseline = Some(actual);
    }
}

#[test]
fn scheduling() {
    deterministic(
        "Invoke.Boolean.Not.True, Invoke.Boolean.Not.False",
        "False, True",
        &[invoke(), source("boolean", "not")],
    );
    let definition = "([Function.Seed] Return.Flower)";
    deterministic(
        &format!("Left.Invoke.Seed.{definition}, Right.Invoke.Seed.{definition}"),
        &format!("Left.Flower.{definition}, Right.Flower.{definition}"),
        &[invoke()],
    );
}
