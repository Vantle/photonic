use crate::lowering::parse;
use crate::prism::{Outcome, Search};
use crate::runtime::Limit;

fn particle(label: &str, count: usize) -> String {
    if count == 0 {
        return "()".into();
    }
    vec![label; count].join(".")
}

fn check(source: &str, target: &str) -> Outcome {
    let mut search = {
        let program = parse(source).unwrap();
        let target = crate::source::Program {
            rule: program.rule.clone(),
            ..parse(target).unwrap()
        };
        Search::new(program, target)
    };
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

#[test]
fn action() {
    for coefficient in 0..=3 {
        for count in 0..=3 {
            let rule = format!("[Repeat] {}", particle("Unit", coefficient));
            let input = particle("Repeat", count);
            let target = particle("Unit", coefficient * count);
            assert_eq!(check(&format!("{input} {rule}"), &target), Outcome::Reached);
            assert_eq!(
                check(
                    &format!("{input} {rule}"),
                    &particle("Unit", coefficient * count + 1)
                ),
                Outcome::Unreachable
            );
            assert_eq!(
                check(
                    &format!(
                        "{} [Repeat] {}",
                        particle("Repeat", coefficient),
                        particle("Unit", count)
                    ),
                    &target
                ),
                Outcome::Reached
            );
            assert_eq!(
                check(&format!("({rule}).{input}"), &format!("({rule}).{target}")),
                Outcome::Reached
            );
            assert_eq!(
                check(&format!("({rule}).{input}"), &target),
                Outcome::Unreachable
            );
            let composed = format!("Add.{input}, Add.Unit {rule} [Add, Add] ()");
            assert_eq!(
                check(&composed, &particle("Unit", coefficient * count + 1)),
                Outcome::Reached
            );
        }
    }
}

#[test]
fn example() {
    for (source, target) in [
        (
            include_str!("../../program/natural/multiplication.wave"),
            include_str!("../../program/natural/six.particle"),
        ),
        (
            include_str!("../../program/natural/composition.wave"),
            include_str!("../../program/natural/seven.particle"),
        ),
        (
            include_str!("../../program/natural/action.wave"),
            include_str!("../../program/natural/retained.particle"),
        ),
    ] {
        assert_eq!(check(source, target), Outcome::Reached);
    }
    assert_eq!(
        check("Add.Add.Unit [Add, Add] ()", "Unit"),
        Outcome::Unreachable
    );
}
