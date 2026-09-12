use molten::lowering::parse;
use molten::obsidian::{Failure, Outcome, Search};
use molten::runtime::Limit;

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
    let program = include_str!("../../../mathematics/natural/addition.lava");
    let target = include_str!("../../../mathematics/natural/result.lava");
    assert_eq!(outcome(program, target), Outcome::Reached);
    assert_eq!(
        outcome(program, "Zero.Successor.Successor.Successor.Successor"),
        Outcome::Unreachable
    );
    let program = include_str!("../../../mathematics/natural/membership.lava");
    assert_eq!(outcome(program, "Natural"), Outcome::Reached);
    assert_eq!(
        outcome(&program.replacen("Zero", "Unknown", 1), "Natural"),
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
