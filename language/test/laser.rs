use super::Laser;
use super::net::{Cycle, Net};
use crate::runtime::{Limit, Runtime};
use crate::stop::Stop;

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

// Whether some configuration holds the atom in a coherence of the root frame.
fn root(state: &[crate::snapshot::Node], atom: &str) -> bool {
    state.iter().any(|node| {
        node.world.iter().any(|world| {
            node.frame[world.frame].parent.is_none()
                && world
                    .particle
                    .iter()
                    .any(|token| crate::test::atom(token) == Some(atom))
        })
    })
}

// A rule value keeps its output beside its coherence even in the frame it captures: `[A] B` rides
// `Out` back into the outer scope and fires there, so `Out` never reaches the root.
#[test]
fn residence() {
    let source = "((P.Z.A, [Q.Z] Out), [P] Q.([A] B), [Q.B] Done)";
    agree(source, 1_000_000, Limit::default());
    let program = frontend::lowering::parse(source).unwrap();
    let mut runtime = Runtime::new(&program);
    runtime.run(1_000_000, Limit::default());
    assert!(!root(&runtime.snapshot().state, "Out"));
    let mut plain = Laser::plain(&program);
    plain.run(1_000_000, Limit::default());
    assert!(plain.closed());
    assert!(!root(&plain.report().state, "Out"));
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
        "((P.Z.A, [Q.Z] Out), [P] Q.([A] B), [Q.B] Done)",
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
        "((P.Z.A, [Q.Z] Out), [P] Q.([A] B), [Q.B] Done)",
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
        "((P.Z.A, [Q.Z] Out), [P] Q.([A] B), [Q.B] Done)",
    ]) {
        let program = frontend::lowering::parse(source).unwrap();
        let mut plain = Laser::plain(&program);
        plain.run(100_000_000, limit);
        let mut net = Net::new(&program).unwrap();
        let explored = net.explore(usize::MAX, limit, Cycle::Find).unwrap();
        assert_eq!(explored.mirrors(&net, &plain), Ok(()), "{source}");
    }
}

// Grounding takes a step of work for each match it reads. A budget too small for a configuration's
// parts stops the net before it expands that configuration, a budget as large as a closed
// exploration's work closes it alike, and parts with millions of matches stop within the budget.
#[test]
fn budget() {
    let limit = Limit::default();
    let explore = |program: &frontend::source::Program, budget: usize, limit: Limit| {
        Net::new(program)
            .unwrap()
            .explore(budget, limit, Cycle::Find)
            .unwrap()
    };
    for source in [
        crate::family::dial(3),
        crate::family::diner(3),
        "A, [A] B, [B] C, [C] A".to_owned(),
        "Go.A, Go.B, [Go] (X, [X] Y), [Y.A] Z, [Y.B, Z] W".to_owned(),
    ] {
        let program = frontend::lowering::parse(&source).unwrap();
        let whole = explore(&program, usize::MAX, limit);
        assert!(whole.closed, "{source}");
        let mut previous = 0;
        for budget in 0..whole.work {
            let explored = explore(&program, budget, limit);
            assert!(!explored.closed, "{source} {budget}");
            assert!(explored.work <= budget, "{source} {budget}");
            assert!(explored.configuration >= previous, "{source} {budget}");
            previous = explored.configuration;
        }
        let exact = explore(&program, whole.work, limit);
        assert!(exact.closed, "{source}");
        assert_eq!(
            (exact.configuration, exact.event, exact.work),
            (whole.configuration, whole.event, whole.work),
            "{source}"
        );
    }
    let program =
        frontend::lowering::parse("[C.C.D] B.A, B, D.([B.A.B, B.B] D.D), [()] B.B.A, [C.D, B.C]")
            .unwrap();
    let small = Limit {
        configuration: 256,
        ..limit
    };
    let explored = explore(&program, 100_000, small);
    assert!(!explored.closed);
    assert!(explored.work <= 100_000);
}

// Picks that differ only in which of several identical inputs took which kind bind the same part,
// so a rule of seven identical inputs over fourteen kinds makes C(20, 7) picks rather than 14^7.
// Past PICK picks or OFFER parts a marking's joins are a host join, which takes a step of work for
// each pick, and a budget too small for it stops the net before the marking.
#[test]
fn join() {
    let wide = |kind: usize, input: usize| {
        let rule = format!("[{}] Y", vec!["X"; input].join(", "));
        (0..kind)
            .map(|index| format!("X.K{index}"))
            .chain([rule])
            .collect::<Vec<_>>()
            .join(", ")
    };
    let limit = Limit {
        configuration: 1,
        ..Limit::default()
    };
    let explore = |source: &str, budget: usize| {
        let program = frontend::lowering::parse(source).unwrap();
        Net::new(&program)
            .unwrap()
            .explore(budget, limit, Cycle::Find)
            .unwrap()
    };
    let free = explore(&wide(12, 2), usize::MAX);
    let costly = [(wide(14, 7), 77_520), (wide(60, 2), 1_830)];
    for (source, pick) in &costly {
        let explored = explore(source, usize::MAX);
        assert!(explored.work >= pick + free.work, "{source}");
        let stopped = explore(source, explored.work - 1);
        assert_eq!(stopped.configuration, 1, "{source}");
        assert_eq!(stopped.event, 0, "{source}");
        assert!(stopped.work < explored.work, "{source}");
        assert!(matches!(stopped.stop[..], [Stop::Work { .. }]), "{source}");
    }
    let program = frontend::lowering::parse(&wide(6, 3)).unwrap();
    let mut net = Net::new(&program).unwrap();
    let whole = net
        .explore(usize::MAX, Limit::default(), Cycle::Find)
        .unwrap();
    let mut plain = Laser::plain(&program);
    plain.run(100_000_000, Limit::default());
    assert_eq!(whole.mirrors(&net, &plain), Ok(()));
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

// A work budget bounds a run exactly and a record budget within the matches of one configuration,
// even where one round would scan, carry and fire far more; running on resumes where a budget
// stopped, so a run in small steps reaches the configurations, events and work of one run.
#[test]
fn allowance() {
    let limit = Limit {
        record: usize::MAX,
        ..Limit::default()
    };
    let many = (0..30)
        .map(|index| format!("A.K{index}"))
        .chain(["[A, A] B".to_owned()])
        .collect::<Vec<_>>()
        .join(", ");
    let program = frontend::lowering::parse(&many).unwrap();
    for record in [1_000, 10_000, 100_000] {
        let mut laser = Laser::new(&program);
        laser.run(usize::MAX, Limit { record, ..limit });
        assert!(!laser.closed(), "{record}");
        assert!(laser.report().record < record + 900, "{record}");
    }
    for source in [
        many.as_str(),
        "S, [T.S, Z.C.T, T.Z.C.S], [()] (T.S, T.Z.C.S, T.Z.S.S, T.Z.A.S)",
    ] {
        let program = frontend::lowering::parse(source).unwrap();
        for budget in [1, 7, 100, 2_000] {
            let mut laser = Laser::new(&program);
            laser.run(budget, limit);
            assert!(laser.summary().work <= budget, "{source} {budget}");
        }
    }
    for source in [
        crate::family::dial(3),
        crate::family::diner(3),
        "A, K, [A] B, [[A] B] C, [C, K] D".to_owned(),
        "Go.Y, K, [K] L, [Go] (X, [X] ().([Y] Z))".to_owned(),
    ] {
        let program = frontend::lowering::parse(&source).unwrap();
        let mut whole = Laser::new(&program);
        whole.run(usize::MAX, limit);
        assert!(whole.closed(), "{source}");
        for budget in [3, 40, 700] {
            let mut stepped = Laser::new(&program);
            while !stepped.closed() {
                stepped.run(budget, limit);
                assert!(stepped.summary().work <= whole.summary().work, "{source}");
            }
            assert_eq!(stepped.summary(), whole.summary(), "{source} {budget}");
        }
    }
}

// An application whose result a round cannot name within what the round has left waits for the
// next round, so a run too small to name it leaves the exploration open, spending exactly its
// budget, and a larger run closes it as one run does. Two steps scan the start and pay for one
// application, which leaves nothing to name its result with.
#[test]
fn deferral() {
    let limit = Limit::default();
    for source in [
        "Seed.X, Seed.X, Seed.X, [Seed] (A, A)",
        "Go, Go, Go, [Go] (X, X, [X] Y)",
    ] {
        let program = frontend::lowering::parse(source).unwrap();
        let mut runtime = Runtime::new(&program);
        runtime.run(100_000_000, limit);
        let mut whole = Laser::new(&program);
        whole.run(100_000_000, limit);
        let mut laser = Laser::new(&program);
        laser.run(2, limit);
        assert!(!laser.round.retry.is_empty(), "{source}");
        assert!(!laser.closed(), "{source}");
        assert_eq!(laser.summary().work, 2, "{source}");
        laser.run(100_000_000, limit);
        assert!(laser.closed(), "{source}");
        assert_eq!(laser.agree(&runtime), Ok(()), "{source}");
        assert_eq!(laser.summary().state, whole.summary().state, "{source}");
        assert_eq!(laser.summary().event, whole.summary().event, "{source}");
    }
}

// Naming results and environments charges each run its steps without taking it past its budget,
// and runs of every size charge alike for any number of workers, reaching the configurations and
// events of one run. A run must hold the steps its largest name takes: these programs need three.
#[test]
fn charge() {
    let executor = crate::executor::Executor::new(std::num::NonZeroUsize::new(4).unwrap()).unwrap();
    let limit = Limit::default();
    for source in [
        "Seed.X, Seed.X, Seed.X, [Seed] (A, A)",
        "Go, Go, Go, [Go] (X, X, [X] Y)",
        "Go.Y, Go.Y, Go.Y, [Go] (X, [X] ().([Y] Z))",
    ] {
        let program = frontend::lowering::parse(source).unwrap();
        let mut whole = Laser::new(&program);
        whole.run(100_000_000, limit);
        assert!(whole.closed(), "{source}");
        for budget in [3, 5, 9, 50] {
            let mut sequential = Laser::new(&program);
            let mut parallel = Laser::new(&program);
            for _ in 0..10_000 {
                let before = sequential.summary().work;
                sequential.run(budget, limit);
                parallel.parallel(&executor, budget, limit);
                assert!(sequential.summary().work - before <= budget, "{source}");
                assert_eq!(
                    parallel.summary(),
                    sequential.summary(),
                    "{source} {budget}"
                );
                if sequential.closed() {
                    break;
                }
            }
            assert!(sequential.closed(), "{source} {budget}");
            assert_eq!(
                (sequential.summary().state, sequential.summary().event),
                (whole.summary().state, whole.summary().event),
                "{source} {budget}"
            );
        }
    }
}

// A trace whose environment takes more steps to name than a round has left waits for the next round
// with every trace after it, spending what the round had left, so every run stays within its
// budget, and runs of every size reach the configurations and events of one run for any number of
// workers. Naming these environments takes no steps, so before each run every environment is
// remembered as taking five.
#[test]
fn environment() {
    let executor = crate::executor::Executor::new(std::num::NonZeroUsize::new(4).unwrap()).unwrap();
    let limit = Limit::default();
    let program = frontend::lowering::parse("Go.Y, Go.Y, [Go] (X, [X] ().([Y] Z))").unwrap();
    let mut whole = Laser::new(&program);
    whole.run(100_000_000, limit);
    assert!(whole.closed());
    let costly = |laser: &mut Laser| {
        laser.environment = super::memo::Memo::default();
        for origin in 0..laser.state.len() {
            for frame in 0..laser.state[origin].frame.len() {
                laser.environment.get((origin, frame), |_| {
                    let mut budget = usize::MAX;
                    let named = laser.state[origin].environment(frame, &mut budget);
                    super::taxonomy::Name::Found(std::sync::Arc::new(named.unwrap()), 5)
                });
            }
        }
    };
    for budget in [7, 10, 16, 25, 60] {
        let mut sequential = Laser::new(&program);
        let mut parallel = Laser::new(&program);
        for _ in 0..10_000 {
            costly(&mut sequential);
            costly(&mut parallel);
            let before = sequential.summary().work;
            sequential.run(budget, limit);
            parallel.parallel(&executor, budget, limit);
            assert!(sequential.summary().work - before <= budget, "{budget}");
            assert_eq!(parallel.summary(), sequential.summary(), "{budget}");
            if sequential.closed() {
                break;
            }
        }
        eprintln!(
            "WORK {budget} stepped {} whole {}",
            sequential.summary().work,
            whole.summary().work
        );
        assert!(sequential.closed(), "{budget}");
        assert!(sequential.summary().work > whole.summary().work, "{budget}");
        assert_eq!(
            (sequential.summary().state, sequential.summary().event),
            (whole.summary().state, whole.summary().event),
            "{budget}"
        );
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
