use crate::lowering::parse;
use crate::path::Search;
use crate::prism::Outcome;
use crate::runtime::Limit;

fn search(source: &str, target: &str) -> Search {
    Search::new(parse(source).unwrap(), parse(target).unwrap()).unwrap()
}

#[test]
fn batching() {
    let particle = ["A"; 8].join(".");
    let content = ["B.C.E"; 6].join(",");
    let delay = ["D"; 8].join(",");
    let world = |stage| format!("{particle},{content},Stage{stage}.C,{delay}");
    let mut source = format!("{} [{particle},B,B,B,B,B.E.E,C] Never ", world(0));
    for stage in 0..8 {
        source.push_str(&format!(
            "[Stage{stage}.C,{delay}] Stage{}.C,{delay} ",
            stage + 1
        ));
    }
    let mut actual = search(&source, &world(8));
    let mut expected = search(&source, &world(8));
    let mut limit = Limit {
        state: 32,
        record: 1_000_000,
        world: 32,
        cell: 64,
        frame: 4,
    };
    for iteration in 0..10_000 {
        let budget = [0, 1, 2, 3, 7, 31, 127][iteration % 7];
        limit.record = if iteration % 13 == 0 { 1 } else { 1_000_000 };
        actual.run(budget, limit);
        for _ in 0..budget {
            expected.run(1, limit);
        }
        assert_eq!(
            serde_json::to_value(actual.report()).unwrap(),
            serde_json::to_value(expected.report()).unwrap()
        );
        assert_eq!(
            serde_json::to_value(actual.statistic()).unwrap(),
            serde_json::to_value(expected.statistic()).unwrap()
        );
        if actual.summary().outcome == Outcome::Reached {
            break;
        }
    }
    assert_eq!(actual.summary().outcome, Outcome::Reached);
}

#[test]
fn execution() {
    for (source, target) in [
        ("A [A] B [B] C", "C"),
        ("A,B [A,B] C", "C"),
        ("A,A,B [A,B] C [A,C] D", "D"),
        ("Seed.A [Seed] [A] B", "B.([A] B)"),
        ("A [A] B,C [B,C] D", "D"),
    ] {
        let mut path = search(source, target);
        path.run(100_000, Limit::default());
        assert_eq!(path.report().outcome, Outcome::Reached, "{source}");
        let mut exhaustive =
            crate::prism::Search::new(parse(source).unwrap(), parse(target).unwrap()).unwrap();
        exhaustive.run(100_000, None);
        assert_eq!(exhaustive.report().outcome, Outcome::Reached);
    }
}

#[test]
fn unknown() {
    for source in ["A", "A [A] B [B] A", "A [A] B [A] C"] {
        let mut path = search(source, "Missing");
        path.run(10_000, Limit::default());
        assert_eq!(path.report().outcome, Outcome::Unknown);
    }
    assert!(Search::new(parse("A").unwrap(), parse("[A] B").unwrap()).is_err());
}

#[test]
fn resume() {
    let source = "A [A] B [B] C";
    let mut complete = search(source, "C");
    complete.run(10_000, Limit::default());
    let expected = serde_json::to_value(complete.report()).unwrap();
    let mut chunk = search(source, "C");
    for _ in 0..10_000 {
        chunk.run(1, Limit::default());
    }
    assert_eq!(serde_json::to_value(chunk.report()).unwrap(), expected);
    for limit in [
        Limit {
            state: 1,
            ..Limit::default()
        },
        Limit {
            record: 1,
            ..Limit::default()
        },
        Limit {
            cell: 0,
            ..Limit::default()
        },
    ] {
        let mut path = search(source, "C");
        path.run(10_000, limit);
        assert_eq!(path.report().outcome, Outcome::Unknown);
        path.run(10_000, Limit::default());
        let mut actual = serde_json::to_value(path.report()).unwrap();
        actual["work"] = expected["work"].clone();
        assert_eq!(actual, expected);
    }
}

#[test]
fn factor() {
    let particle = (0..16)
        .map(|index| format!("Value{index}"))
        .collect::<Vec<_>>()
        .join(".");
    let mut source = format!("{particle},Stage0.B [{particle},B] Never\n");
    for index in 0..32 {
        source.push_str(&format!("[Stage{index}] Stage{}\n", index + 1));
    }
    let target = format!("{particle},Stage32.B");
    let limit = Limit {
        cell: 32,
        ..Limit::default()
    };
    let mut complete = search(&source, &target);
    complete.run(10000, limit);
    assert_eq!(complete.summary().outcome, Outcome::Reached);
    let expected = serde_json::to_value(complete.report()).unwrap();
    for interval in [1, 7, 19] {
        let mut chunk = search(&source, &target);
        for iteration in 0..1000 {
            chunk.run(1, limit);
            if iteration % interval == 0 {
                chunk.run(
                    1,
                    Limit {
                        record: 1,
                        ..Limit::default()
                    },
                );
            }
            if chunk.summary().outcome == Outcome::Reached {
                break;
            }
        }
        assert_eq!(serde_json::to_value(chunk.report()).unwrap(), expected);
    }
}

#[test]
fn inference() {
    let source = "Seed.A [Seed] [A] B";
    let mut path = search(source, "Seed.B");
    path.run(100_000, Limit::default());
    assert_eq!(path.report().outcome, Outcome::Unknown);
    let mut exhaustive =
        crate::prism::Search::new(parse(source).unwrap(), parse("Seed.B").unwrap()).unwrap();
    exhaustive.run(100_000, None);
    assert_eq!(exhaustive.report().outcome, Outcome::Reached);
}

#[test]
fn metadata() {
    for source in [
        "A [A] B",
        "A.X,B.X [A,B] C",
        "Seed.A [Seed] [A] B",
        "A [A] (B [B] C)",
    ] {
        let mut runtime = crate::runtime::Runtime::new(parse(source).unwrap());
        for _ in 0..10 {
            runtime.run(100_000, None);
            let Some(event) = runtime.first() else {
                break;
            };
            let snapshot = runtime.snapshot();
            let expected = &snapshot.event[0];
            assert_eq!(event.target, expected.target);
            assert_eq!(event.rule, expected.rule);
            assert_eq!(
                event.binding.footprint.iter().copied().collect::<Vec<_>>(),
                expected.footprint
            );
            assert_eq!(
                event.binding.exact.iter().copied().collect::<Vec<_>>(),
                expected.exact
            );
            assert_eq!(
                event.binding.read.iter().copied().collect::<Vec<_>>(),
                expected.read
            );
            let next = runtime.state[event.target].clone();
            runtime = crate::runtime::Runtime::seed(runtime.program.clone(), next);
        }
    }
}

#[test]
fn summary() {
    for (source, target) in [("A [A] B", "B"), ("A", "Missing"), ("A", "A")] {
        let mut path = search(source, target);
        path.run(100_000, Limit::default());
        let summary = path.summary();
        let report = path.report();
        assert_eq!(summary.outcome, report.outcome);
        assert_eq!(summary.event, report.event.len());
        assert_eq!(summary.work, report.work);
        assert_eq!(
            serde_json::to_value(summary.witness).unwrap(),
            serde_json::to_value(report.witness.map(|index| &report.state[index])).unwrap()
        );
    }
}

#[test]
fn current() {
    let mut path = search("A [A] B", "Missing");
    assert_eq!(path.current().world[0].particle[0].display, "A");
    path.run(10000, Limit::default());
    assert_eq!(path.summary().outcome, Outcome::Unknown);
    assert!(path.summary().witness.is_none());
    assert_eq!(path.current().world[0].particle[0].display, "B");
    assert_eq!(
        serde_json::to_value(path.current()).unwrap(),
        serde_json::to_value(path.report().state.last()).unwrap()
    );
}

#[test]
fn inspection() {
    let mut path = search("A [A] B [B] A", "Missing");
    path.run(10000, Limit::default());
    let report = path.report();
    for state in &report.state {
        assert_eq!(
            serde_json::to_value(path.inspect(state.id)).unwrap(),
            serde_json::to_value(state).unwrap()
        );
    }
    for (index, event) in report.event.iter().enumerate() {
        assert_eq!(
            serde_json::to_value(path.transition(index)).unwrap(),
            serde_json::to_value(event).unwrap()
        );
    }
    assert!(path.inspect(report.state.len()).is_none());
    assert!(path.transition(report.event.len()).is_none());
}

#[test]
fn collision() {
    let source = "X.A [X] (),()";
    let target = "A,A";
    let mut path = search(source, target);
    let mut previous = 0;
    for _ in 0..1000 {
        path.run(1, Limit::default());
        let summary = path.summary();
        assert!(summary.work - previous <= 1);
        previous = summary.work;
    }
    assert_eq!(path.summary().outcome, Outcome::Unknown);
    assert_eq!(path.current().world.len(), 2);
    let mut exhaustive =
        crate::prism::Search::new(parse(source).unwrap(), parse(target).unwrap()).unwrap();
    exhaustive.run(10000, None);
    assert_eq!(exhaustive.report().outcome, Outcome::Unreachable);
}

#[test]
fn suspension() {
    for (source, target, limit) in [
        (
            "A [A] B,C [B,C] D",
            "D",
            Limit {
                world: 1,
                ..Limit::default()
            },
        ),
        (
            "A [A] (B [B] (C [C] D))",
            "D",
            Limit {
                frame: 1,
                ..Limit::default()
            },
        ),
    ] {
        let mut complete = search(source, target);
        complete.run(100_000, Limit::default());
        let mut expected = serde_json::to_value(complete.report()).unwrap();
        assert_eq!(complete.summary().outcome, Outcome::Reached);
        let mut paused = search(source, target);
        paused.run(100_000, limit);
        assert_eq!(paused.summary().outcome, Outcome::Unknown);
        for _ in 0..100_000 {
            paused.current();
            paused.run(1, Limit::default());
            let summary = paused.summary();
            if summary.event > 0 {
                paused.transition(summary.event - 1).unwrap();
            }
            if summary.outcome == Outcome::Reached {
                break;
            }
        }
        let mut actual = serde_json::to_value(paused.report()).unwrap();
        actual.as_object_mut().unwrap().remove("work");
        expected.as_object_mut().unwrap().remove("work");
        assert_eq!(actual, expected, "{source}");
    }
}
