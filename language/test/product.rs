use crate::prism::Outcome;
use crate::runtime::{Limit, Runtime};
use frontend::lowering::parse;
use frontend::source::{Program, Value};

fn program(left: usize, right: usize) -> Program {
    let mut program = parse(include_str!("../../program/natural/product.wave")).unwrap();
    for (particle, count) in program.initial.iter_mut().zip([left, right]) {
        particle.retain(|value| *value != Value::Atom("Unit".into()));
        particle.extend(std::iter::repeat_n(Value::Atom("Unit".into()), count));
    }
    program
}

fn target(left: usize, right: usize, result: usize) -> Program {
    parse(&format!(
        "Archive{},Archive{},Product{}",
        ".Unit".repeat(left),
        ".Unit".repeat(right),
        ".Unit".repeat(result)
    ))
    .unwrap()
}

#[test]
fn boundary() {
    for left in 0..=1 {
        for right in 0..=1 {
            for result in 0..=2 {
                let program = program(left, right);
                let target = {
                    let mut target = target(left, right, result);
                    target.preserve(&program);
                    target
                };
                let mut runtime = Runtime::new(&program);
                runtime.run(
                    3_000_000,
                    Limit {
                        configuration: 10_000,
                        record: 2_000_000,
                        occurrence: 64,
                        scope: 40,
                        coherence: 10,
                    },
                );
                let verdict = runtime.verdict(&target);
                let report = runtime.snapshot();
                assert!(report.closed, "{left} * {right}");
                assert_eq!(
                    verdict.outcome,
                    if result == left * right {
                        Outcome::Reached
                    } else {
                        Outcome::Unreachable
                    },
                    "{left} * {right} = {result}"
                );
            }
        }
    }
}

#[test]
fn composition() {
    let mut program = program(1, 1);
    program
        .initial
        .push(vec![Value::Atom("Add".into()), Value::Atom("Unit".into())]);
    program
        .rule
        .extend(parse("[Product,Add] Product").unwrap().rule);
    let mut goal = target(1, 1, 2);
    goal.preserve(&program);
    let mut runtime = Runtime::new(&program);
    runtime.run(
        3_000_000,
        Limit {
            configuration: 10_000,
            record: 2_000_000,
            occurrence: 64,
            scope: 40,
            coherence: 10,
        },
    );
    assert_eq!(runtime.verdict(&goal).outcome, Outcome::Reached);
}
