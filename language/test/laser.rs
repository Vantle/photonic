use super::Laser;
use crate::runtime::{Limit, Runtime};

// A deduction walks from its event's source, each event starting where the one before it ends.
fn walk(laser: &Laser, source: &str) {
    let report = laser.report();
    for transition in &report.event {
        assert_eq!(
            transition.deduction.is_empty(),
            !transition.inferred,
            "{source}"
        );
        let mut at = transition.source;
        for &step in &transition.deduction {
            assert_eq!(report.event[step].source, at, "{source}");
            at = report.event[step].target;
        }
    }
}

pub(crate) fn agree(source: &str, budget: usize, limit: Limit) {
    let program = frontend::lowering::parse(source).unwrap();
    let mut runtime = Runtime::new(&program);
    runtime.run(budget, limit);
    let mut laser = Laser::new(&program);
    laser.run(budget, limit);
    assert!(runtime.closed(), "the interpreter did not close {source}");
    assert_eq!(laser.agree(&runtime), Ok(()), "{source}");
    walk(&laser, source);
}

#[test]
fn agreement() {
    for source in [
        "A, [A] B",
        "A, A, [A] B",
        "A.B, A.C, [A, B] X, [A, C] Y",
        "A.A.A.A, [A.A] B",
        "A, K, [A] B, [[A] B] C, [C, K] D",
        "B, [B] C, [C] B",
        "[A.A] Z, B, [B] C, [C] B, (A, [Q] R)",
        "Seed.Extra, [Seed] A, [A] Result",
        "A, [A] (B, [B] C)",
        "A,B, [A] (C, [C,B] D)",
        "Seed.A.X, [Seed] (([A] B), ([A] C))",
        "A.([A] B),A.([A] C)",
        "A,A,A, [A,A] B",
        "A.X, [A] (B, C), [X, X] X, [X.B.C] Done",
        "Go.Y, K, [K] L, [Go] (X, [X] ().([Y] Z))",
        "A, [A] (X, [X] Y), [Y] Z",
        "Go.Y, [Go] (X, [X] ().([Y] Z))",
        "Go.K, [Go] ().([K] A)",
        "A, Key, [A] B, [B, Key] (C, [Q] R)",
    ] {
        agree(source, 1_000_000, Limit::default());
    }
}

#[test]
fn family() {
    for count in [1, 2, 3, 4] {
        agree(&crate::family::dial(count), 10_000_000, Limit::default());
    }
    let limit = Limit {
        record: usize::MAX,
        ..Limit::default()
    };
    for count in [2, 3, 4] {
        agree(&crate::family::diner(count), 100_000_000, limit);
    }
}

#[test]
fn open() {
    for source in ["A.K, [A] B, [B] (C, [C] D)", "Go, [Go] ().([] A)"] {
        let program = frontend::lowering::parse(source).unwrap();
        let mut runtime = Runtime::new(&program);
        runtime.run(1_000_000, Limit::default());
        let mut laser = Laser::new(&program);
        laser.run(1_000_000, Limit::default());
        assert!(!runtime.closed(), "{source}");
        assert!(!laser.closed(), "{source}");
    }
}

#[test]
fn worker() {
    let executor = crate::executor::Executor::new(std::num::NonZeroUsize::new(4).unwrap()).unwrap();
    let limit = Limit {
        record: usize::MAX,
        ..Limit::default()
    };
    for source in [
        crate::family::dial(4),
        crate::family::diner(3),
        "A, K, [A] B, [[A] B] C, [C, K] D".to_owned(),
        "Go.Y, K, [K] L, [Go] (X, [X] ().([Y] Z))".to_owned(),
    ] {
        let program = frontend::lowering::parse(&source).unwrap();
        let mut runtime = Runtime::new(&program);
        runtime.run(100_000_000, limit);
        let mut parallel = Laser::new(&program);
        parallel.parallel(&executor, 100_000_000, limit);
        assert_eq!(parallel.agree(&runtime), Ok(()), "{source}");
        let mut sequential = Laser::new(&program);
        sequential.run(100_000_000, limit);
        assert_eq!(parallel.summary(), sequential.summary(), "{source}");
        assert!(
            parallel.state.iter().eq(sequential.state.iter()),
            "{source}"
        );
    }
}

#[test]
fn resume() {
    let limit = Limit {
        record: usize::MAX,
        ..Limit::default()
    };
    let small = Limit {
        configuration: 10,
        ..limit
    };
    for source in [
        crate::family::dial(3),
        crate::family::diner(3),
        "A, K, [A] B, [[A] B] C, [C, K] D".to_owned(),
    ] {
        let program = frontend::lowering::parse(&source).unwrap();
        let mut runtime = Runtime::new(&program);
        runtime.run(100_000_000, limit);
        let mut laser = Laser::new(&program);
        laser.run(100_000_000, small);
        assert!(!laser.closed(), "{source}");
        laser.run(100_000_000, limit);
        assert_eq!(laser.agree(&runtime), Ok(()), "{source}");
        let peak = laser.report().peak;
        for record in [1, peak / 2] {
            let mut laser = Laser::new(&program);
            laser.run(100_000_000, Limit { record, ..limit });
            assert!(!laser.closed(), "{source} {record}");
            assert!(laser.report().record >= record, "{source} {record}");
            laser.run(100_000_000, limit);
            assert_eq!(laser.agree(&runtime), Ok(()), "{source} {record}");
            assert_eq!(laser.report().peak, peak, "{source} {record}");
        }
    }
}

#[test]
fn verdict() {
    for (source, target, budget) in [
        ("A.B", "B.A", 0),
        ("A, [A] B", "B", 0),
        ("A, [A] B", "B", 12_000),
        ("A, [A] B, [B] A", "C", 12_000),
        ("Seed.Extra, [Seed] A, [A] Result", "Result.Extra", 12_000),
        ("[A.A] Z, B, (A, [Q] R)", "B, (A, [Q] R)", 12_000),
        (
            "[A.A] Z, B, [B] C, [C] B, (A, [Q] R)",
            "B, (A, [Q] R)",
            12_000,
        ),
        (
            "Pair.Seed, Pair.Other, [Seed] Intermediate, [Intermediate] Kind, [Other] Kind, \
             [Pair.Kind, Pair.Kind] ([()] Result)",
            "Result.Seed.Other",
            12_000,
        ),
        (
            "Pair.Seed, Pair.Other, [Seed] Kind, [Pair.Kind, Pair.Kind] ([()] Result)",
            "Result.Seed.Other",
            12_000,
        ),
        ("A", "A.A", 12_000),
        ("A.Extra, [A] B", "B", 12_000),
        ("A.Extra, [A] B", "B.Extra", 12_000),
        ("Enter, [Enter] (Goal, [Missing] Done)", "Goal", 12_000),
        ("Seed.X, [Seed] (A, B)", "A.X, B.X", 12_000),
        ("$x, [$y] Result", "Result", 12_000),
        ("().([A.B] C)", "().([B.A] C)", 12_000),
        ("A, [A] B, [B] C, [C] A", "C", 12_000),
    ] {
        let program = frontend::lowering::parse(source).unwrap();
        let target = crate::test::target(&program, target);
        let mut runtime = Runtime::new(&program);
        runtime.run(budget, Limit::default());
        let mut laser = Laser::new(&program);
        laser.run(budget, Limit::default());
        let expected = runtime.verdict(&target);
        let observed = laser.verdict(&target);
        assert_eq!(observed.outcome, expected.outcome, "{source}");
        assert_eq!(
            observed.witness.is_some(),
            expected.witness.is_some(),
            "{source}"
        );
    }
}
