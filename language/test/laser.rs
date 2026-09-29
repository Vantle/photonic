use super::Laser;
use super::net::{Cycle, Net};
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

// A plain exploration is the part of the full one that matched events reach, and has no inferred
// event.
#[test]
fn plain() {
    let limit = Limit {
        record: usize::MAX,
        ..Limit::default()
    };
    let family = [
        crate::family::dial(3),
        crate::family::diner(3),
        "A, [A] B, [B] C, [C] A".to_owned(),
        "Case.(([P] True), ([P] False)), [Claim] Done, Claim".to_owned(),
    ];
    for source in family.iter().map(String::as_str).chain([
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
        "A.X, [A] (B, C), [X, X] X, [X.B.C] Done",
        "Go.Y, K, [K] L, [Go] (X, [X] ().([Y] Z))",
        "Go.Y, [Go] (X, [X] ().([Y] Z))",
        "A, Key, [A] B, [B, Key] (C, [Q] R)",
    ]) {
        let program = frontend::lowering::parse(source).unwrap();
        let mut full = Laser::new(&program);
        full.run(100_000_000, limit);
        let mut plain = Laser::plain(&program);
        plain.run(100_000_000, limit);
        assert_eq!(plain.within(&full), Ok(()), "{source}");
        assert_eq!(plain.summary().inferred, 0, "{source}");
    }
}

// A run ends where no event leaves its configuration, and goes on forever around a cycle; runs that
// meet again after different orders are neither.
#[test]
fn ending() {
    for (source, end, endless) in [
        ("A, [A] B", 1, false),
        ("A, [A] B, [A] C", 2, false),
        ("A, B, [A] C, [B] D", 1, false),
        ("B, [B] C, [C] B", 0, true),
        ("A, [A] B, [A] C, [C] A", 1, true),
    ] {
        let program = frontend::lowering::parse(source).unwrap();
        let mut laser = Laser::plain(&program);
        laser.run(1_000_000, Limit::default());
        assert!(laser.closed(), "{source}");
        let ending = laser.ending();
        assert_eq!(ending.end.len(), end, "{source}");
        assert_eq!(ending.endless, endless, "{source}");
    }
}

// A reduced exploration keeps every configuration where a plain run ends and every cycle, firing one
// coherence or scope at a time when its events commute with the rest, and in any number of workers;
// a rule that can consume a live rule turns the reduction off.
#[test]
fn reduction() {
    let executor = crate::executor::Executor::new(std::num::NonZeroUsize::new(4).unwrap()).unwrap();
    let limit = Limit {
        record: usize::MAX,
        ..Limit::default()
    };
    let family = [
        (crate::family::dial(3), true),
        (crate::family::diner(3), false),
    ];
    for (source, smaller) in family
        .iter()
        .map(|(source, smaller)| (source.as_str(), *smaller))
        .chain([
            ("Case.(([P] True), ([P] False)), [Claim] Done, Claim", false),
            ("A, [A] B, [A] C, [C] A", false),
            ("B, [B] C, [C] B", false),
            ("Go.Y, K, [K] L, [Go] (X, [X] ().([Y] Z))", true),
            ("A, Key, [A] B, [B, Key] (C, [Q] R)", false),
            ("Go, Go, [Go] (X, [X] Y)", true),
            ("Go.A, Go.B, [Go] (X, [X] Y), [Y.A] Z, [Y.B, Z] W", true),
            ("Go, Go, [Go] (X, [Q] R), [X] Y, [Y] X", true),
            ("Go, Go, Go, [Go] (X, [X] Y, [X] W)", true),
            ("Go, Go, [Go] (X, [Q] R), [X] Y, [X] W, [Y, W] Done", true),
            ("Go, Go, [X] Y, [Go] (A), [A.([X] Y)] Z", false),
        ])
    {
        let program = frontend::lowering::parse(source).unwrap();
        let mut plain = Laser::plain(&program);
        plain.run(100_000_000, limit);
        let mut reduced = Laser::reduced(&program);
        reduced.run(100_000_000, limit);
        assert_eq!(reduced.preserves(&plain), Ok(()), "{source}");
        assert_eq!(
            reduced.summary().state < plain.summary().state,
            smaller,
            "{source}"
        );
        let mut parallel = Laser::reduced(&program);
        parallel.parallel(&executor, 100_000_000, limit);
        assert!(parallel.state.iter().eq(reduced.state.iter()), "{source}");
    }
}

// The net explores every schedule of plain events from tables of parts, and closes with the plain
// engine's configurations, events, end configurations and cycles.
#[test]
fn net() {
    let limit = Limit {
        record: usize::MAX,
        ..Limit::default()
    };
    let family = [
        crate::family::dial(3),
        crate::family::dial(4),
        crate::family::diner(3),
        crate::family::diner(4),
    ];
    for source in family.iter().map(String::as_str).chain([
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
        "Case.(([P] True), ([P] False)), [Claim] Done, Claim",
        "A, [A] B, [A] C, [C] A",
        "Go, Go, [Go] (X, [X] Y)",
        "Go.A, Go.B, [Go] (X, [X] Y), [Y.A] Z, [Y.B, Z] W",
        "Go, Go, [Go] (X, [Q] R), [X] Y, [Y] X",
        "Go, Go, Go, [Go] (X, [X] Y, [X] W)",
        "Go, Go, [Go] (X, [Q] R), [X] Y, [X] W, [Y, W] Done",
        "Go, Go, [X] Y, [Go] (A), [A.([X] Y)] Z",
        "X.([A, C] B).A, C, C",
        "A, A, A, [A, A, A] B",
        "A.B, A.B, A, [A, A.B] C",
        "Claim, [Claim] P.Work, [Work] Done, [P] X",
    ]) {
        let program = frontend::lowering::parse(source).unwrap();
        let mut plain = Laser::plain(&program);
        plain.run(100_000_000, limit);
        let mut net = Net::new(&program).unwrap();
        let explored = net.explore(limit, Cycle::Find).unwrap();
        assert_eq!(explored.mirrors(&net, &plain), Ok(()), "{source}");
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

// Exploring a step at a time under limits that rise retries the identities each limit blocked, while
// new traces keep arriving; one they identify meanwhile waits for its retry instead of firing twice.
#[test]
fn retry() {
    let limit = Limit {
        configuration: 256,
        ..Limit::default()
    };
    for source in [
        "D.([A.D, C.B.B] C.D.B), A.C.A, B, [A] ([B.A.C] B.B.C, B, B.C, D, C)",
        "(A.D.B.([A]), [A.C, D.B] A, [B.C] (), C.B.D, [A.D]), [D.D, D.A] C.C.D, A.A, [B.D] (B.B.A, A.D.C), [A.D.B] D.B.C",
        "([B, A.B.B] A.B.C, [C.B.B] B, [()], B.A.([B.A] ())), B.B.([C, D] ()), ([B, C.D.C], A.C.([()]), [A.C] (A.C.D, C)), A.D.A.([C] A.D), A.D.([D, C] ())",
    ] {
        let program = frontend::lowering::parse(source).unwrap();
        let mut runtime = Runtime::new(&program);
        runtime.run(200_000, limit);
        assert!(runtime.closed(), "{source}");
        let mut laser = Laser::new(&program);
        for configuration in [2, 4, 8, 16] {
            for _ in 0..2 {
                laser.run(
                    1,
                    Limit {
                        configuration,
                        ..limit
                    },
                );
            }
        }
        laser.run(2_000_000, limit);
        assert_eq!(laser.agree(&runtime), Ok(()), "{source}");
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
