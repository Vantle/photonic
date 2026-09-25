use crate::lowering::parse;
use crate::prism::Outcome;
use crate::runtime::Limit;

fn check(source: &str, target: &str) {
    let mut path = {
        let program = parse(source).unwrap();
        let target = crate::source::Program {
            rule: program.rule.clone(),
            ..parse(target).unwrap()
        };
        crate::path::Search::new(program, target)
    };
    let mut exhaustive = {
        let program = parse(source).unwrap();
        let target = crate::source::Program {
            rule: program.rule.clone(),
            ..parse(target).unwrap()
        };
        crate::prism::Search::new(program, target)
    };
    let limit = Limit {
        state: 16,
        world: 16,
        cell: 32,
        ..Limit::default()
    };
    for _ in 0..10_000 {
        path.run(1, limit);
        exhaustive.run(7, Some(limit));
        if path.summary().outcome == Outcome::Reached
            && exhaustive.verdict().outcome == Outcome::Reached
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
            let mut path = {
                let program = parse(&source).unwrap();
                let target = crate::source::Program {
                    rule: program.rule.clone(),
                    ..parse("Result").unwrap()
                };
                crate::path::Search::new(program, target)
            };
            path.run(10_000, Limit::default());
            assert_eq!(path.summary().outcome, Outcome::Unknown, "{source}");
            let mut exhaustive = {
                let program = parse(&source).unwrap();
                let target = crate::source::Program {
                    rule: program.rule.clone(),
                    ..parse("Result").unwrap()
                };
                crate::prism::Search::new(program, target)
            };
            exhaustive.run(10_000, None);
            assert_eq!(
                exhaustive.verdict().outcome,
                Outcome::Unreachable,
                "{source}"
            );
        }
    }
}
