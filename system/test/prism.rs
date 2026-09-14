use crate::lowering::parse;
use crate::prism::{Failure, Outcome, Search};
use crate::runtime::Limit;

fn search(program: &str, target: &str) -> Search {
    Search::new(parse(program).unwrap(), parse(target).unwrap()).unwrap()
}

fn outcome(program: &str, target: &str) -> Outcome {
    let mut search = search(program, target);
    search.run(12_000, None);
    search.report().outcome
}

#[test]
fn reachability() {
    let initial = search("A.B", "B.A").report();
    assert_eq!(initial.outcome, Outcome::Reached);
    assert_eq!(initial.witness, Some(0));
    assert_eq!(outcome("A [A] B", "B"), Outcome::Reached);
    assert_eq!(outcome("A [A] B [B] A", "C"), Outcome::Unreachable);
    assert_eq!(
        outcome("Seed.Extra [Seed] A [A] Result", "Result.Extra"),
        Outcome::Reached
    );
}

#[test]
fn abstraction() {
    for initial in ["Pair.Seed, Pair.Other", "Pair.Other, Pair.Seed"] {
        let program = format!(
            "{initial} [Seed] Intermediate [Intermediate] Kind [Other] Kind \
             [Pair.Kind, Pair.Kind] ([] Result)"
        );
        let mut search = search(&program, "Result.Seed.Other");
        search.run(12_000, None);
        let report = search.report();
        assert!(report.execution.closed);
        assert_eq!(report.outcome, Outcome::Reached);
    }
    assert_eq!(
        outcome(
            "Pair.Seed, Pair.Other [Seed] Kind [Pair.Kind, Pair.Kind] ([] Result)",
            "Result.Seed.Other",
        ),
        Outcome::Unreachable,
    );
    assert_eq!(
        outcome(
            "Pair.Pair.Seed.Other [Seed] Kind [Other] Kind \
             [Pair.Kind, Pair.Kind] ([] Result)",
            "Result.Seed.Other",
        ),
        Outcome::Unreachable,
    );
}

#[test]
fn identity() {
    assert_eq!(outcome("A", "A.A"), Outcome::Unreachable);
    assert_eq!(outcome("A.Extra [A] B", "B"), Outcome::Unreachable);
    assert_eq!(outcome("A.Extra [A] B", "B.Extra"), Outcome::Reached);
    assert_eq!(
        outcome("Enter [Enter] (Goal [Missing] Done)", "Goal"),
        Outcome::Unreachable
    );
    assert_eq!(
        outcome("Seed.X [Seed] (A, B)", "A.X, B.X"),
        Outcome::Unreachable
    );
    assert_eq!(outcome("$x [$y] Result", "Result"), Outcome::Unreachable);
    assert_eq!(
        search("([A.B] C)", "([B.A] C)").report().outcome,
        Outcome::Reached
    );
}

#[test]
fn uncertainty() {
    let mut paused = search("A [A] B", "B");
    paused.run(0, None);
    assert_eq!(paused.report().outcome, Outcome::Unknown);
    paused.run(
        12_000,
        Some(Limit {
            state: 1,
            ..Limit::default()
        }),
    );
    assert_eq!(paused.report().outcome, Outcome::Unknown);
    paused.run(12_000, Some(Limit::default()));
    assert_eq!(paused.report().outcome, Outcome::Reached);
}

#[test]
fn target() {
    assert!(matches!(
        Search::new(parse("A").unwrap(), parse("B [B] A").unwrap()),
        Err(Failure::Declaration)
    ));
    assert_eq!(outcome("A [A]", ""), Outcome::Reached);
    assert_eq!(outcome("A [A] ()", ""), Outcome::Unreachable);
}

#[test]
fn arithmetic() {
    let program = include_str!("../../mathematics/natural/addition.wave");
    let target = include_str!("../../mathematics/natural/result.particle");
    assert_eq!(outcome(program, target), Outcome::Reached);
    assert_eq!(
        outcome(program, "Unit.Unit.Unit.Unit"),
        Outcome::Unreachable
    );
    let program = include_str!("../../mathematics/natural/membership.wave");
    assert_eq!(outcome(program, "Natural"), Outcome::Reached);
    assert_eq!(
        outcome(&program.replacen("Check", "Unknown", 1), "Natural"),
        Outcome::Unreachable
    );
}

#[test]
fn metaprogramming() {
    assert_eq!(outcome("Seed.A [Seed] [A] B", "Seed.B"), Outcome::Reached);
    assert_eq!(
        outcome("([A] B).A [[A] B] [A] C", "([A] C).C"),
        Outcome::Reached
    );
    assert_eq!(
        outcome("Seed.A [Seed] [A] B [[[A] B]] Missing", "Missing"),
        Outcome::Unreachable
    );
    assert_eq!(
        outcome(
            "Not.True [True] Boolean [False] Boolean [Not.Boolean] ([True] False [False] True)",
            "False"
        ),
        Outcome::Reached
    );
}

#[test]
fn concept() {
    assert_eq!(outcome("Not.True", "False"), Outcome::Unreachable);
    assert_eq!(
        outcome("Not.True [Not.True] False", "False"),
        Outcome::Reached
    );
    assert_eq!(
        outcome("Not.True [Not.True] Ready", "Ready"),
        Outcome::Reached
    );
    assert_eq!(outcome("True.False", "True.False"), Outcome::Reached);
}

#[test]
fn indexing() {
    let source = "Use.Seed [Seed] Kind [Use.Kind] ([Seed] Done)";
    let mut clean = search(source, "Done");
    clean.run(100_000, None);
    let clean = clean.report();
    assert_eq!(clean.outcome, Outcome::Reached);
    let noise = (0..1000)
        .map(|index| format!("[Absent{index}] Unused{index} "))
        .collect::<String>();
    let mut indexed = search(&format!("{source} {noise}"), "Done");
    indexed.run(100_000, None);
    let indexed = indexed.report();
    assert!(indexed.execution.closed);
    assert_eq!(indexed.outcome, Outcome::Reached);
    assert_eq!(
        serde_json::to_value(indexed.execution.state).unwrap(),
        serde_json::to_value(clean.execution.state).unwrap()
    );
    assert_eq!(outcome("A,B [A,B] C", "C"), Outcome::Reached);
    assert_eq!(
        outcome("([A] B).A [[A] B] [A] C", "([A] C).C"),
        Outcome::Reached
    );
}

#[test]
fn verdict() {
    let mut search = search("A [A] B [B] C", "C");
    assert_eq!(search.verdict().outcome, Outcome::Unknown);
    search.run(12000, None);
    let report = search.report();
    assert_eq!(search.verdict().outcome, report.outcome);
    assert_eq!(search.verdict().witness, report.witness);
    let work = report.execution.work;
    for (target, expected) in [
        ("A", Outcome::Reached),
        ("D", Outcome::Unreachable),
        ("B", Outcome::Reached),
    ] {
        search.target(parse(target).unwrap()).unwrap();
        assert_eq!(search.verdict().outcome, expected);
        assert_eq!(search.report().execution.work, work);
    }
    assert!(search.target(parse("B [B] C").unwrap()).is_err());
    assert_eq!(search.verdict().outcome, Outcome::Reached);
    search.target(parse("D").unwrap()).unwrap();
    assert_eq!(search.verdict().witness, None);
}
