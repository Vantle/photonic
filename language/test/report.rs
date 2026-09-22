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
        let mut actual =
            crate::path::Search::new(parse(source).unwrap(), parse(target).unwrap()).unwrap();
        let mut expected =
            crate::path::Search::new(parse(source).unwrap(), parse(target).unwrap()).unwrap();
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
        let mut prism =
            crate::prism::Search::new(parse(source).unwrap(), parse("C").unwrap()).unwrap();
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
        let mut actual = crate::path::Search::new(source.clone(), target.clone()).unwrap();
        let mut expected = crate::path::Search::new(source, target).unwrap();
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
