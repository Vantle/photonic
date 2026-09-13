use crate::lowering::parse;
use crate::obsidian::{Outcome, Search};
use crate::runtime::Limit;

fn numeral(count: usize) -> String {
    if count == 0 {
        return "()".into();
    }
    vec!["Unit"; count].join(".")
}

fn operand(label: &str, count: usize) -> String {
    format!("{label}{}", ".Unit".repeat(count))
}

fn check(source: &str, target: &str) -> Outcome {
    let mut search = Search::new(parse(source).unwrap(), parse(target).unwrap()).unwrap();
    search.run(
        12_000,
        Some(Limit {
            cell: 32,
            ..Limit::default()
        }),
    );
    let report = search.report();
    assert!(report.execution.closed, "{source}");
    report.outcome
}

fn sum(left: usize, right: usize) -> String {
    format!(
        "{}, {} [Add, Add] ()",
        operand("Add", left),
        operand("Add", right)
    )
}

#[test]
fn identity() {
    for left in 0..=4 {
        for right in 0..=4 {
            let target = numeral(left + right);
            assert_eq!(check(&sum(left, right), &target), Outcome::Reached);
            assert_eq!(check(&sum(right, left), &target), Outcome::Reached);
            assert_eq!(
                check(&sum(left, right), &numeral(left + right + 1)),
                Outcome::Unreachable
            );
        }
    }
}

#[test]
fn successor() {
    for left in 0..=3 {
        for right in 0..=3 {
            let target = numeral(left + right + 1);
            assert_eq!(check(&sum(left + 1, right), &target), Outcome::Reached);
            assert_eq!(check(&sum(left, right + 1), &target), Outcome::Reached);
            let source = format!(
                "{}, {} [Add, Add] Step [Step] Unit",
                operand("Add", left),
                operand("Add", right)
            );
            assert_eq!(check(&source, &target), Outcome::Reached);
        }
    }
}

#[test]
fn associativity() {
    for left in 0..=2 {
        for middle in 0..=2 {
            for right in 0..=2 {
                let initial = format!(
                    "{}, {}, {}",
                    operand("Left", left),
                    operand("Middle", middle),
                    operand("Right", right)
                );
                let target = numeral(left + middle + right);
                for rule in [
                    "[Left, Middle] Partial [Partial, Right] ()",
                    "[Middle, Right] Partial [Left, Partial] ()",
                ] {
                    assert_eq!(
                        check(&format!("{initial} {rule}"), &target),
                        Outcome::Reached
                    );
                }
            }
        }
    }
}

#[test]
fn independence() {
    let source = "Seed.Unit [Seed] (Add, Add) [Add, Add] ()";
    assert_eq!(check(source, "Unit"), Outcome::Reached);
    assert_eq!(check(source, "Unit.Unit"), Outcome::Unreachable);
}

#[test]
fn example() {
    assert_eq!(
        check(
            include_str!("../../mathematics/natural/sum.wave"),
            include_str!("../../mathematics/natural/ten.particle")
        ),
        Outcome::Reached
    );
    let source = include_str!("../../mathematics/decimal/addition.wave");
    assert_eq!(
        check(
            source,
            include_str!("../../mathematics/decimal/result.particle")
        ),
        Outcome::Reached
    );
    assert_eq!(
        check(source, "Coefficient.Unit, Scale"),
        Outcome::Unreachable
    );
    assert_eq!(
        check(
            source,
            &format!("Coefficient.{}, Scale.Place.Place", numeral(10))
        ),
        Outcome::Unreachable
    );
}
