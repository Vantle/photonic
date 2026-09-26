use crate::prism::{Outcome, Verdict};
use crate::runtime::{Limit, Runtime};
use frontend::lowering::parse;

struct Case {
    runtime: Runtime,
    target: frontend::source::Program,
}

impl Case {
    fn new(program: &str, target: &str) -> Self {
        let program = parse(program).unwrap();
        Self {
            target: crate::test::target(&program, target),
            runtime: Runtime::new(&program),
        }
    }

    fn verdict(&self) -> Verdict {
        self.runtime.verdict(&self.target)
    }

    fn report(&self) -> serde_json::Value {
        serde_json::to_value((self.verdict(), self.runtime.snapshot())).unwrap()
    }
}

#[test]
fn observation() {
    for (program, target) in [
        ("A, [A] B, [B] C, [C] A", "A"),
        ("Seed.A, [Seed] ().([A] B)", "Seed.A"),
        ("A, [A] B, [A] C, [B,C] Forbidden", "A"),
    ] {
        let mut observed = Case::new(program, target);
        for budget in 1usize..=128 {
            observed.runtime.run(1, Limit::default());
            if budget.is_power_of_two() {
                let mut reference = Case::new(program, target);
                reference.runtime.run(budget, Limit::default());
                assert_eq!(observed.report(), reference.report());
            }
        }
    }
}

fn outcome(program: &str, target: &str) -> Outcome {
    let mut case = Case::new(program, target);
    case.runtime.run(12_000, Limit::default());
    case.verdict().outcome
}

#[test]
fn reachability() {
    let initial = Case::new("A.B", "B.A").verdict();
    assert_eq!(initial.outcome, Outcome::Reached);
    assert_eq!(initial.witness, Some(0));
    assert_eq!(outcome("A, [A] B", "B"), Outcome::Reached);
    assert_eq!(outcome("A, [A] B, [B] A", "C"), Outcome::Unreachable);
    assert_eq!(
        outcome("Seed.Extra, [Seed] A, [A] Result", "Result.Extra"),
        Outcome::Reached
    );
}

#[test]
fn abstraction() {
    for initial in ["Pair.Seed, Pair.Other", "Pair.Other, Pair.Seed"] {
        let program = format!(
            "{initial}, [Seed] Intermediate, [Intermediate] Kind, [Other] Kind, \
             [Pair.Kind, Pair.Kind] ([()] Result)"
        );
        let mut case = Case::new(&program, "Result.Seed.Other");
        case.runtime.run(12_000, Limit::default());
        assert!(case.runtime.snapshot().closed);
        assert_eq!(case.verdict().outcome, Outcome::Reached);
    }
    assert_eq!(
        outcome(
            "Pair.Seed, Pair.Other, [Seed] Kind, [Pair.Kind, Pair.Kind] ([()] Result)",
            "Result.Seed.Other",
        ),
        Outcome::Unreachable,
    );
    assert_eq!(
        outcome(
            "Pair.Pair.Seed.Other, [Seed] Kind, [Other] Kind, \
             [Pair.Kind, Pair.Kind] ([()] Result)",
            "Result.Seed.Other",
        ),
        Outcome::Unreachable,
    );
}

#[test]
fn identity() {
    assert_eq!(outcome("A", "A.A"), Outcome::Unreachable);
    assert_eq!(outcome("A.Extra, [A] B", "B"), Outcome::Unreachable);
    assert_eq!(outcome("A.Extra, [A] B", "B.Extra"), Outcome::Reached);
    assert_eq!(
        outcome("Enter, [Enter] (Goal, [Missing] Done)", "Goal"),
        Outcome::Unreachable
    );
    assert_eq!(
        outcome("Seed.X, [Seed] (A, B)", "A.X, B.X"),
        Outcome::Unreachable
    );
    assert_eq!(outcome("$x, [$y] Result", "Result"), Outcome::Unreachable);
    assert_eq!(
        Case::new("().([A.B] C)", "().([B.A] C)").verdict().outcome,
        Outcome::Reached
    );
}

#[test]
fn uncertainty() {
    let mut paused = Case::new("A, [A] B", "B");
    paused.runtime.run(0, Limit::default());
    assert_eq!(paused.verdict().outcome, Outcome::Unknown);
    paused.runtime.run(
        12_000,
        Limit {
            configuration: 1,
            ..Limit::default()
        },
    );
    assert_eq!(paused.verdict().outcome, Outcome::Unknown);
    paused.runtime.run(12_000, Limit::default());
    assert_eq!(paused.verdict().outcome, Outcome::Reached);
}

#[test]
fn target() {
    let mut case = Case::new("A", "B, [B] A");
    case.runtime.run(12_000, Limit::default());
    assert_eq!(case.verdict().outcome, Outcome::Unreachable);
    assert_eq!(outcome("A, [A]", ""), Outcome::Reached);
    assert_eq!(outcome("A, [A] ()", ""), Outcome::Unreachable);
}

#[test]
fn arithmetic() {
    let program = include_str!("../../program/natural/addition.wave");
    let target = include_str!("../../program/natural/result.particle");
    assert_eq!(outcome(program, target), Outcome::Reached);
    assert_eq!(
        outcome(program, "Unit.Unit.Unit.Unit"),
        Outcome::Unreachable
    );
    let program = include_str!("../../program/natural/membership.wave");
    assert_eq!(outcome(program, "Natural"), Outcome::Reached);
    assert_eq!(
        outcome(&program.replacen("Check", "Unknown", 1), "Natural"),
        Outcome::Unreachable
    );
}

#[test]
fn partition() {
    for (program, target, expected) in [
        ("A, [A] [B]", "B", Outcome::Reached),
        ("B, [A] [B]", "A", Outcome::Reached),
        ("A, [A] [B] C", "C", Outcome::Reached),
        ("B, [A] [B] C", "C", Outcome::Reached),
        ("C, [A] [B] C", "A", Outcome::Unreachable),
        ("A.B, [A] [B] C", "C.C", Outcome::Reached),
        ("A, [A] [B] C", "D", Outcome::Unreachable),
    ] {
        assert_eq!(outcome(program, target), expected, "{program} to {target}");
    }
}

#[test]
fn metaprogramming() {
    assert_eq!(
        outcome("Seed.A, [Seed] ().([A] B)", "Seed.B"),
        Outcome::Reached
    );
    assert_eq!(
        outcome("([A] B).A, [[A] B] ().([A] C)", "([A] C).C"),
        Outcome::Reached
    );
    assert_eq!(
        outcome("Seed.A, [Seed] ().([A] B), [[[A] B]] Missing", "Missing"),
        Outcome::Unreachable
    );
    assert_eq!(
        outcome(
            "Not.True, [True] Boolean, [False] Boolean, [Not.Boolean] ([True] False, [False] True)",
            "False"
        ),
        Outcome::Reached
    );
}

#[test]
fn concept() {
    assert_eq!(outcome("Not.True", "False"), Outcome::Unreachable);
    assert_eq!(
        outcome("Not.True, [Not.True] False", "False"),
        Outcome::Reached
    );
    assert_eq!(
        outcome("Not.True, [Not.True] Ready", "Ready"),
        Outcome::Reached
    );
    assert_eq!(outcome("True.False", "True.False"), Outcome::Reached);
}

#[test]
fn indexing() {
    let source = "Use.Seed, [Seed] Kind, [Use.Kind] ([Seed] Done)";
    let mut clean = Case::new(source, "Done");
    clean.runtime.run(100_000, Limit::default());
    assert_eq!(clean.verdict().outcome, Outcome::Reached);
    let clean = clean.runtime.snapshot();
    let noise = (0..1000)
        .map(|index| format!(", [Absent{index}] Unused{index}"))
        .collect::<String>();
    let mut indexed = Case::new(&format!("{source}{noise}"), "Done");
    indexed.runtime.run(
        100_000,
        Limit {
            occurrence: 1100,
            ..Limit::default()
        },
    );
    assert_eq!(indexed.verdict().outcome, Outcome::Reached);
    let indexed = indexed.runtime.snapshot();
    assert!(indexed.closed);
    assert_eq!(indexed.state.len(), clean.state.len());
    assert_eq!(indexed.event.len(), clean.event.len());
    assert!(
        indexed
            .state
            .iter()
            .all(|state| state.frame[0].particle.len() == 1002)
    );
    assert!(
        clean
            .state
            .iter()
            .all(|state| state.frame[0].particle.len() == 2)
    );
    assert_eq!(outcome("A,B, [A,B] C", "C"), Outcome::Reached);
    assert_eq!(
        outcome("([A] B).A, [[A] B] ().([A] C)", "([A] C).C"),
        Outcome::Reached
    );
}

#[test]
fn verdict() {
    let mut case = Case::new("A, [A] B, [B] C", "C");
    assert_eq!(case.verdict().outcome, Outcome::Unknown);
    case.runtime.run(12000, Limit::default());
    let verdict = case.verdict();
    assert_eq!(verdict.outcome, Outcome::Reached);
    assert_eq!(case.verdict(), verdict);
    let work = case.runtime.snapshot().work;
    for (target, expected) in [
        ("A", Outcome::Reached),
        ("D", Outcome::Unreachable),
        ("B", Outcome::Reached),
    ] {
        let target = frontend::source::Program {
            rule: parse("[A] B, [B] C").unwrap().rule,
            ..parse(target).unwrap()
        };
        assert_eq!(case.runtime.verdict(&target).outcome, expected);
        assert_eq!(case.runtime.snapshot().work, work);
    }
    let unruled = case.runtime.verdict(&parse("B, [B] C").unwrap());
    assert_eq!(unruled.outcome, Outcome::Unreachable);
    assert_eq!(case.runtime.verdict(&parse("D").unwrap()).witness, None);
}
