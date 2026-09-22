use crate::lowering::parse;
use crate::runtime::{Limit, Runtime};
use serde::Serialize;
use std::io::{Error, Write};

fn compare(actual: &impl Serialize, expected: &impl Serialize) {
    let compact = serde_json::to_vec(expected).unwrap();
    let pretty = serde_json::to_vec_pretty(expected).unwrap();
    for _ in 0..2 {
        assert_eq!(serde_json::to_vec(actual).unwrap(), compact);
        assert_eq!(serde_json::to_vec_pretty(actual).unwrap(), pretty);
    }
}

#[test]
fn resume() {
    for (source, target) in [
        ("", ""),
        ("A [A] B [B] C", "C"),
        ("A [A] B [B] A", "Missing"),
        ("A [A] (B [B] (C [C] D))", "D"),
        ("Seed.A [Seed] [A] B", "B.([A] B)"),
        ("A.X,B.X [A,B] C", "C.X,X"),
    ] {
        let mut actual = {
            let program = parse(source).unwrap();
            let target = crate::source::Program {
                rule: program.rule.clone(),
                ..parse(target).unwrap()
            };
            crate::path::Search::new(program, target)
        };
        let mut expected = {
            let program = parse(source).unwrap();
            let target = crate::source::Program {
                rule: program.rule.clone(),
                ..parse(target).unwrap()
            };
            crate::path::Search::new(program, target)
        };
        for budget in [0, 1, 2, 7, 31, 128] {
            for record in [1, 1_000_000] {
                let limit = Limit {
                    record,
                    ..Limit::default()
                };
                actual.run(budget, limit);
                expected.run(budget, limit);
                compare(&actual.view(), &expected.report());
                compare(&actual.statistic(), &expected.statistic());
            }
        }
        let owned = actual.report();
        drop(actual);
        compare(&owned, &expected.report());
    }
}

#[test]
fn exhaustive() {
    for source in [
        "",
        "A [A] B [B] C",
        "A [A] B [A] C [B,C] Forbidden",
        "Seed.A [Seed] [A] B",
        "A [A] (B [B] (C [C] D))",
    ] {
        let mut actual = Runtime::new(parse(source).unwrap());
        let mut expected = Runtime::new(parse(source).unwrap());
        let mut prism = {
            let program = parse(source).unwrap();
            let target = crate::source::Program {
                rule: program.rule.clone(),
                ..parse("C").unwrap()
            };
            crate::prism::Search::new(program, target)
        };
        for budget in [0, 1, 2, 7, 31, 128] {
            actual.run(budget, None);
            expected.run(budget, None);
            prism.run(budget, None);
            compare(&actual.view(), &expected.snapshot());
            compare(&prism.view(), &prism.report());
        }
        let owned = actual.snapshot();
        drop(actual);
        compare(&owned, &expected.snapshot());
    }
}

struct Writer {
    remaining: usize,
}

impl Write for Writer {
    fn write(&mut self, value: &[u8]) -> std::io::Result<usize> {
        if self.remaining == 0 {
            return Err(Error::other("full"));
        }
        let count = value.len().min(self.remaining);
        self.remaining -= count;
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn failure() {
    for remaining in [0, 1, 8, 64, 512, 1024] {
        let source = parse("A [A] (B [B] (C [C] D))").unwrap();
        let target = parse("D").unwrap();
        let mut actual = {
            let program = source.clone();
            let target = crate::source::Program {
                rule: program.rule.clone(),
                ..target.clone()
            };
            crate::path::Search::new(program, target)
        };
        let mut expected = {
            let program = source;
            let target = crate::source::Program {
                rule: program.rule.clone(),
                ..target
            };
            crate::path::Search::new(program, target)
        };
        actual.run(31, Limit::default());
        expected.run(31, Limit::default());
        let mut writer = Writer { remaining };
        assert!(
            serde_json::to_writer(&mut writer, &actual.view())
                .unwrap_err()
                .is_io()
        );
        compare(&actual.view(), &expected.report());
        actual.run(128, Limit::default());
        expected.run(128, Limit::default());
        compare(&actual.view(), &expected.report());
        compare(&actual.statistic(), &expected.statistic());
    }
}

#[test]
fn catalog() {
    let program = crate::lowering::parse("[A] B [A] B [([A] B)] C").unwrap();
    let mut runtime = crate::runtime::Runtime::new(program);
    runtime.run(100_000, None);
    let value = serde_json::to_value(runtime.view()).unwrap();
    assert_eq!(value["definition"].as_array().unwrap().len(), 2);
    assert_eq!(value, serde_json::to_value(runtime.snapshot()).unwrap());
    for state in value["state"].as_array().unwrap() {
        for frame in state["frame"].as_array().unwrap() {
            for token in frame["particle"].as_array().unwrap() {
                assert!(token.get("display").is_none());
                assert!(
                    value["definition"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|definition| definition["label"] == token["label"])
                );
                assert_eq!(token["capture"], 0);
            }
        }
    }
}
