use crate::lowering::parse;
use crate::obsidian::{Outcome, Search};
use crate::runtime::Limit;
use crate::source::{Program, Value};

fn program(left: usize, right: usize) -> Program {
    let mut program = parse(include_str!("../../mathematics/natural/product.wave")).unwrap();
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
                let mut search =
                    Search::new(program(left, right), target(left, right, result)).unwrap();
                search.run(
                    3_000_000,
                    Some(Limit {
                        state: 10_000,
                        record: 2_000_000,
                        cell: 64,
                        frame: 40,
                        world: 10,
                    }),
                );
                let report = search.report();
                assert!(report.execution.closed, "{left} * {right}");
                assert_eq!(
                    report.outcome,
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
    let mut search = Search::new(program, target(1, 1, 2)).unwrap();
    search.run(
        3_000_000,
        Some(Limit {
            state: 10_000,
            record: 2_000_000,
            cell: 64,
            frame: 40,
            world: 10,
        }),
    );
    assert_eq!(search.report().outcome, Outcome::Reached);
}
