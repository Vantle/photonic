use crate::prism::Outcome;
use crate::runtime::{Limit, Runtime};
use frontend::lowering::parse;

fn check(source: &str, target: &str) {
    let program = parse(source).unwrap();
    let goal = crate::test::target(&program, target);
    let mut path = crate::path::Search::new(program.clone(), Some(goal.clone()));
    let mut exhaustive = Runtime::new(&program);
    let limit = Limit {
        configuration: 16,
        coherence: 16,
        occurrence: 32,
        ..Limit::default()
    };
    for _ in 0..10_000 {
        path.run(1, limit);
        exhaustive.run(7, limit);
        if path.summary().outcome == Outcome::Reached
            && exhaustive.verdict(&goal).outcome == Outcome::Reached
        {
            return;
        }
    }
    panic!("{source} did not reach {target}");
}

#[test]
fn expansion() {
    for input in 1..=6 {
        let initial = vec!["()"; input].join(",");
        let pattern = vec!["()"; input].join(",");
        check(&format!("{initial}, [{pattern}]"), "");
        for output in 1..=4 {
            let target = (0..output)
                .map(|position| format!("Value{position}"))
                .collect::<Vec<_>>()
                .join(",");
            check(&format!("{initial}, [{pattern}] ({target})"), &target);
            let target = vec!["()"; output].join(",");
            check(&format!("{initial}, [{pattern}] ({target})"), &target);
        }
    }
}

#[test]
fn mixture() {
    for width in 1..=6 {
        let mut initial = vec!["()"; width];
        initial[0] = "Gate.Extra";
        for position in 0..width {
            let mut pattern = vec!["()"; width];
            pattern[position] = "Gate";
            check(
                &format!("{}, [{}] Result", initial.join(","), pattern.join(",")),
                "Result.Extra",
            );
        }
    }
}

#[test]
fn remainder() {
    for width in 1..=6 {
        let initial = (0..width)
            .map(|position| format!("Value{position}"))
            .collect::<Vec<_>>();
        check(
            &format!(
                "{}, [{}] Result",
                initial.join(","),
                vec!["()"; width].join(",")
            ),
            &format!("Result.{}", initial.join(".")),
        );
    }
}

#[test]
fn absence() {
    for width in 1..=6 {
        for count in 0..width {
            let source = format!(
                "[{}] Result, {}",
                vec!["()"; width].join(","),
                vec!["()"; count].join(",")
            );
            let program = parse(&source).unwrap();
            let goal = crate::test::target(&program, "Result");
            let mut path = crate::path::Search::new(program.clone(), Some(goal.clone()));
            path.run(10_000, Limit::default());
            assert_eq!(path.summary().outcome, Outcome::Unknown, "{source}");
            let mut exhaustive = Runtime::new(&program);
            exhaustive.run(10_000, Limit::default());
            assert_eq!(
                exhaustive.verdict(&goal).outcome,
                Outcome::Unreachable,
                "{source}"
            );
        }
    }
}
