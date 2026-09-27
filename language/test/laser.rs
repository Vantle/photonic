use super::Laser;
use crate::runtime::{Limit, Runtime};

pub(crate) fn agree(source: &str, budget: usize, limit: Limit) {
    let program = frontend::lowering::parse(source).unwrap();
    let mut runtime = Runtime::new(&program);
    runtime.run(budget, limit);
    let mut laser = Laser::new(&program);
    laser.run(budget, limit);
    assert!(runtime.closed(), "the interpreter did not close {source}");
    assert_eq!(laser.agree(&runtime), Ok(()), "{source}");
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
    }
}
