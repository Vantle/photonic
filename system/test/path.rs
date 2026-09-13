use crate::lowering::parse;
use crate::obsidian::Outcome;
use crate::path::Search;
use crate::runtime::Limit;

fn search(source: &str, target: &str) -> Search {
    Search::new(parse(source).unwrap(), parse(target).unwrap()).unwrap()
}

#[test]
fn execution() {
    for (source, target) in [
        ("A [A] B [B] C", "C"),
        ("A,B [A,B] C", "C"),
        ("Seed.A [Seed] [A] B", "B.([A] B)"),
        ("A [A] B,C [B,C] D", "D"),
    ] {
        let mut path = search(source, target);
        path.run(100_000, Limit::default());
        assert_eq!(path.report().outcome, Outcome::Reached, "{source}");
        let mut exhaustive =
            crate::obsidian::Search::new(parse(source).unwrap(), parse(target).unwrap()).unwrap();
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
fn inference() {
    let source = "Seed.A [Seed] [A] B";
    let mut path = search(source, "Seed.B");
    path.run(100_000, Limit::default());
    assert_eq!(path.report().outcome, Outcome::Unknown);
    let mut exhaustive =
        crate::obsidian::Search::new(parse(source).unwrap(), parse("Seed.B").unwrap()).unwrap();
    exhaustive.run(100_000, None);
    assert_eq!(exhaustive.report().outcome, Outcome::Reached);
}
