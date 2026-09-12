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
    let initial = search("A.B;", "B.A;").report();
    assert_eq!(initial.outcome, Outcome::Reached);
    assert_eq!(initial.witness, Some(0));
    assert_eq!(outcome("A; [A] -> B;", "B;"), Outcome::Reached);
    assert_eq!(
        outcome("A; [A] -> B; [B] -> A;", "C;"),
        Outcome::Unreachable
    );
    assert_eq!(
        outcome(
            "Seed.Extra; [Seed] -> Box(A); [Box($value)] -> Result($value);",
            "Result(A).Extra;"
        ),
        Outcome::Reached
    );
}

#[test]
fn identity() {
    assert_eq!(outcome("A;", "A.A;"), Outcome::Unreachable);
    assert_eq!(outcome("A.Extra; [A] -> B;", "B;"), Outcome::Unreachable);
    assert_eq!(outcome("A.Extra; [A] -> B;", "B.Extra;"), Outcome::Reached);
    assert_eq!(
        outcome("Enter; [Enter] -> { Goal; };", "Goal;"),
        Outcome::Unreachable
    );
    assert_eq!(
        outcome("Seed.X; [Seed] -> A, B;", "A.X, B.X;"),
        Outcome::Unreachable
    );
    assert_eq!(
        search("@([$value] -> $value);", "@([$other] -> $other);")
            .report()
            .outcome,
        Outcome::Reached
    );
}

#[test]
fn uncertainty() {
    let mut paused = search("A; [A] -> B;", "B;");
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
    let mut circular = search("Seed; [Seed] unless [P] -> P;", "P;");
    circular.run(12_000, None);
    let report = circular.report();
    assert!(report.execution.closed);
    assert_eq!(report.outcome, Outcome::Unknown);
    assert_eq!(report.witness, None);
    assert_eq!(
        outcome("Seed; [Seed] unless [Q] -> P; [Seed] -> Q;", "P;"),
        Outcome::Unreachable
    );
}

#[test]
fn target() {
    assert!(matches!(
        Search::new(parse("A;").unwrap(), parse("B; [B] -> A;").unwrap()),
        Err(Failure::Declaration)
    ));
    assert!(matches!(
        Search::new(parse("A;").unwrap(), parse("Box($value);").unwrap()),
        Err(Failure::Variable)
    ));
    assert_eq!(outcome("A; [A] -> [];", ""), Outcome::Reached);
    assert_eq!(outcome("A; [A] -> ();", ""), Outcome::Unreachable);
}

#[test]
fn arithmetic() {
    let program = include_str!("../../../mathematics/natural/addition.lava");
    let target = include_str!("../../../mathematics/natural/result.lava");
    assert_eq!(outcome(program, target), Outcome::Reached);
    assert_eq!(
        outcome(program, "Successor(Successor(Successor(Successor(Zero))));"),
        Outcome::Unreachable
    );
}
