use crate::lowering::parse;
use crate::path::Search;
use crate::prism::Outcome;
use crate::runtime::{Limit, Runtime};
use crate::support::Status;

fn limit() -> Limit {
    Limit {
        state: 256,
        record: 2_000_000,
        world: 16,
        cell: 256,
        frame: 128,
    }
}

fn tower(depth: usize) -> (String, String) {
    let mut rule = "Done".to_owned();
    let mut target = vec![rule.clone()];
    for position in (0..depth).rev() {
        rule = format!("[Step{position}] {rule}");
        if position > 0 {
            target.push(format!("({rule})"));
        }
    }
    let initial = (0..depth)
        .map(|position| format!("Step{position}"))
        .collect::<Vec<_>>()
        .join(".");
    (format!("{initial} {rule}"), target.join("."))
}

fn execute(source: &str, target: &str) -> Search {
    let mut search = Search::new(parse(source).unwrap(), parse(target).unwrap()).unwrap();
    search.run(1_000_000, limit());
    assert_eq!(search.summary().outcome, Outcome::Reached, "{source}");
    search
}

#[test]
fn generation() {
    for depth in [1, 2, 3, 8, 16, 32] {
        let (source, target) = tower(depth);
        let complete = execute(&source, &target);
        assert_eq!(complete.summary().event, depth);
        let mut chunk = Search::new(parse(&source).unwrap(), parse(&target).unwrap()).unwrap();
        for _ in 0..1_000_000 {
            chunk.run(1, limit());
            if chunk.summary().outcome == Outcome::Reached {
                break;
            }
        }
        assert_eq!(
            serde_json::to_value(chunk.report()).unwrap(),
            serde_json::to_value(complete.report()).unwrap(),
            "depth {depth}"
        );
    }
}

#[test]
fn structure() {
    for depth in [1, 2, 3, 8, 16, 32] {
        let mut value = "Done".to_owned();
        let mut other = "Wrong".to_owned();
        for _ in 0..depth {
            value = format!("[Absent] ({value})");
            other = format!("[Absent] ({other})");
        }
        let source = format!("({value}) [({other})] Forbidden [({value})] Accepted");
        let search = execute(&source, "Accepted");
        assert_eq!(search.summary().event, 1);
        assert!(
            search
                .report()
                .state
                .iter()
                .all(|state| state.world.iter().all(|world| {
                    world
                        .particle
                        .iter()
                        .all(|token| token.label != "Forbidden")
                }))
        );
    }
}

#[test]
fn evidence() {
    for depth in 1..=4 {
        let (source, _) = tower(depth);
        let mut complete = Runtime::new(parse(&source).unwrap());
        complete.run(1_000_000, Some(limit()));
        assert!(complete.closed(), "depth {depth}");
        let expected = complete.snapshot();
        assert!(expected.state.iter().any(|state| {
            state.status == Status::Supported
                && state
                    .world
                    .iter()
                    .any(|world| world.particle.iter().any(|token| token.label == "Done"))
        }));
        let mut chunk = Runtime::new(parse(&source).unwrap());
        chunk.run(0, Some(limit()));
        for _ in 0..1_000_000 {
            chunk.run(1, None);
            if chunk.closed() {
                break;
            }
        }
        assert_eq!(
            serde_json::to_value(chunk.snapshot()).unwrap(),
            serde_json::to_value(expected).unwrap(),
            "depth {depth}"
        );
    }
}

#[test]
fn recursion() {
    for source in [
        "Seed [Seed] (Again [Again] Again)",
        "Seed [Seed] (First [First] Second [Second] First)",
        "Again [Again] Again.([Absent] Done)",
    ] {
        let mut search = Search::new(parse(source).unwrap(), parse("Missing").unwrap()).unwrap();
        search.run(10_000, limit());
        assert_eq!(search.summary().outcome, Outcome::Unknown);
        assert!(search.summary().work <= 10_000);
        assert!(search.report().state.iter().all(|state| {
            state
                .world
                .iter()
                .all(|world| world.particle.iter().all(|token| token.label != "Done"))
        }));
    }
}

#[test]
fn capture() {
    for depth in [1, 2, 3, 8, 16, 32] {
        let mut body = "[A] Local [Make] [Call] (A [Local] Done)".to_owned();
        for position in (1..depth).rev() {
            body = format!("Enter{position} [Enter{position}] ({body})");
        }
        let source = format!("Enter0.Make.Call [Enter0] ({body}) [A] Global");
        let mut search = Search::new(parse(&source).unwrap(), parse("Missing").unwrap()).unwrap();
        let bound = Limit {
            cell: (depth + 4) * (depth + 4),
            ..limit()
        };
        search.run(1_000_000, bound);
        let report = search.report();
        assert!(
            report.state.iter().any(|state| {
                state
                    .world
                    .iter()
                    .any(|world| world.particle.iter().any(|token| token.label == "Done"))
            }),
            "depth {depth}"
        );
        assert!(
            report.state.iter().all(|state| {
                state
                    .world
                    .iter()
                    .all(|world| world.particle.iter().all(|token| token.label != "Global"))
            }),
            "depth {depth}"
        );
        if depth == 32 {
            let mut paused =
                Search::new(parse(&source).unwrap(), parse("Missing").unwrap()).unwrap();
            paused.run(1_000_000, limit());
            let current = paused.current();
            let cell = current
                .world
                .iter()
                .map(|world| world.particle.len())
                .sum::<usize>()
                + current
                    .frame
                    .iter()
                    .map(|frame| frame.held.len())
                    .sum::<usize>();
            assert_eq!(cell, limit().cell);
            paused.run(1_000_000, bound);
            let mut actual = serde_json::to_value(paused.report()).unwrap();
            let mut expected = serde_json::to_value(report).unwrap();
            actual.as_object_mut().unwrap().remove("work");
            expected.as_object_mut().unwrap().remove("work");
            assert_eq!(actual, expected);
        }
    }
}
