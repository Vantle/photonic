use crate::prism::Outcome;
use crate::runtime::{Limit, Runtime};
use frontend::lowering::parse;
use frontend::source::{Program, Value};

fn numeral(count: usize) -> Vec<Value> {
    vec![Value::Atom("Unit".into()); count]
}

fn target(particle: Vec<Value>) -> Program {
    Program {
        initial: vec![particle],
        rule: Vec::new(),
    }
}

fn outcome(program: Program, mut target: Program) -> Outcome {
    target.preserve(&program);
    let mut runtime = Runtime::new(&program);
    runtime.run(12_000, Limit::default());
    assert!(runtime.snapshot().closed);
    runtime.verdict(&target).outcome
}

#[test]
fn identity() {
    let zero = parse(include_str!("../../program/natural/zero.particle")).unwrap();
    assert_eq!(outcome(zero, target(Vec::new())), Outcome::Reached);
    assert_eq!(
        outcome(Program::default(), target(Vec::new())),
        Outcome::Unreachable
    );
    for left in 0..=6 {
        for right in 0..=6 {
            assert_eq!(
                outcome(target(numeral(left)), target(numeral(right))),
                if left == right {
                    Outcome::Reached
                } else {
                    Outcome::Unreachable
                }
            );
        }
    }
}

#[test]
fn successor() {
    let rule = parse(include_str!("../../program/natural/successor.wave"))
        .unwrap()
        .rule;
    for count in 0..=6 {
        let mut input = numeral(count);
        input.push(Value::Atom("Step".into()));
        let program = Program {
            initial: vec![input],
            rule: rule.clone(),
        };
        assert_eq!(
            outcome(program.clone(), target(numeral(count + 1))),
            Outcome::Reached
        );
        assert_eq!(
            outcome(program, target(numeral(count))),
            Outcome::Unreachable
        );
    }
}

#[test]
fn membership() {
    let rule = parse(include_str!("../../program/natural/membership.wave"))
        .unwrap()
        .rule;
    let check = |mut particle: Vec<Value>| {
        particle.push(Value::Atom("Check".into()));
        outcome(
            Program {
                initial: vec![particle],
                rule: rule.clone(),
            },
            parse("Natural").unwrap(),
        )
    };
    for count in 0..=6 {
        assert_eq!(check(numeral(count)), Outcome::Reached);
    }
    for source in [
        "Other",
        "Natural",
        "Check",
        "Unit.Other",
        "Unit.Natural",
        "Unit.Check",
        "0",
        "Zero",
        "Successor",
    ] {
        let particle = parse(source).unwrap().initial.remove(0);
        assert_eq!(check(particle), Outcome::Unreachable, "{source}");
    }
}
