use crate::lowering::parse;
use crate::prism::Outcome;
use crate::runtime::Limit;

fn check(source: &str, target: &str) {
    let mut path =
        crate::path::Search::new(parse(source).unwrap(), parse(target).unwrap()).unwrap();
    let mut exhaustive =
        crate::prism::Search::new(parse(source).unwrap(), parse(target).unwrap()).unwrap();
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
        let pattern = ",".repeat(input - 1);
        for output in 0..=4 {
            let target = (0..output)
                .map(|position| format!("Value{position}"))
                .collect::<Vec<_>>()
                .join(",");
            check(&format!("{initial} [{pattern}] {target}"), &target);
            if output > 0 {
                let target = vec!["()"; output].join(",");
                check(&format!("{initial} [{pattern}] {target}"), &target);
            }
        }
    }
}

#[test]
fn mixture() {
    for width in 1..=6 {
        let mut initial = vec!["()"; width];
        initial[0] = "Gate.Extra";
        for position in 0..width {
            let mut pattern = vec![""; width];
            pattern[position] = "Gate";
            check(
                &format!("{} [{}] Result", initial.join(","), pattern.join(",")),
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
            &format!("{} [{}] Result", initial.join(","), ",".repeat(width - 1)),
            &format!("Result.{}", initial.join(".")),
        );
    }
}

#[test]
fn absence() {
    for width in 1..=6 {
        for count in 0..width {
            let source = format!(
                "{} [{}] Result",
                vec!["()"; count].join(","),
                ",".repeat(width - 1)
            );
            let mut path =
                crate::path::Search::new(parse(&source).unwrap(), parse("Result").unwrap())
                    .unwrap();
            path.run(10_000, Limit::default());
            assert_eq!(path.summary().outcome, Outcome::Unknown, "{source}");
            let mut exhaustive =
                crate::prism::Search::new(parse(&source).unwrap(), parse("Result").unwrap())
                    .unwrap();
            exhaustive.run(10_000, None);
            assert_eq!(
                exhaustive.verdict().outcome,
                Outcome::Unreachable,
                "{source}"
            );
        }
    }
}
