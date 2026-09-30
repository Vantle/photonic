use crate::engine::Engine;
use crate::tuning::Tuning;
use photonic::family;
use photonic::laser::Laser;
use photonic::laser::net::{Cycle, Net};
use photonic::runtime::Limit;

// Every tuning explores exactly as the host's net does, number for number: the same configurations,
// events, end configurations in the same order, cycles and work, whether it counts a few markings
// at a time, decides a few candidates at a time or writes markings across many small segments, and
// under every limit, including a configuration limit reached in the middle of a pass, and every
// budget, including one that runs out in the middle of a window. Both nets ground parts in the same
// order, so even their kinds are numbered alike.
#[test]
fn identical() {
    let tuning = [
        Tuning::default(),
        Tuning {
            window: 3,
            pass: 7,
            first: 16,
            largest: 64,
            initial: 1,
            join: 1,
        },
        Tuning {
            window: 17,
            pass: 61,
            first: 40,
            largest: 300,
            initial: 5,
            join: 3,
        },
        Tuning {
            window: 5,
            pass: 11,
            first: 32,
            largest: 128,
            initial: 2,
            ..Tuning::default()
        },
    ];
    let open = Limit {
        record: usize::MAX,
        ..Limit::default()
    };
    let limit = [
        open,
        Limit {
            configuration: 1,
            ..open
        },
        Limit {
            configuration: 2,
            ..open
        },
        Limit {
            configuration: 17,
            ..open
        },
        Limit {
            configuration: 100,
            ..open
        },
        Limit {
            coherence: 3,
            ..open
        },
        Limit {
            occurrence: 12,
            ..open
        },
        Limit { scope: 1, ..open },
    ];
    let program = [
        family::dial(5),
        family::diner(4),
        family::task(6),
        "A.B, A.C, [A, B] X, [A, C] Y".to_owned(),
        "A, K, [A] B, [[A] B] C, [C, K] D".to_owned(),
        "A, [A] A.A".to_owned(),
        "Go, Go, [Go] (X, [X] Y)".to_owned(),
        "X.([A, C] B).A, C, C".to_owned(),
        "A, A, A, [A, A, A] B".to_owned(),
        "Go.A, Go.B, [Go] (X, [X] Y), [Y.A] Z, [Y.B, Z] W".to_owned(),
        "S, [S] E, [S] F, [S] G, [S] X, [X] Y, [Y] X".to_owned(),
    ];
    for tuning in tuning {
        let Some(engine) = Engine::tuned(tuning).unwrap() else {
            return;
        };
        for source in &program {
            let parsed = frontend::lowering::parse(source).unwrap();
            let mut whole = Net::new(&parsed).unwrap();
            let work = whole.explore(usize::MAX, open, Cycle::Find).unwrap().work;
            let budget = [0, 1, work / 3, work / 2, work.saturating_sub(1), work];
            let case = limit
                .iter()
                .map(|&limit| (usize::MAX, limit))
                .chain(budget.map(|budget| (budget, open)));
            for (budget, limit) in case {
                let mut theirs = Net::new(&parsed).unwrap();
                let expected = theirs.explore(budget, limit, Cycle::Find).unwrap();
                let mut net = Net::new(&parsed).unwrap();
                let explored = engine
                    .explore(&mut net, budget, limit, Cycle::Find)
                    .unwrap();
                let name = format!("{source} {tuning:?} {budget} {limit:?}");
                assert_eq!(explored.agrees(&net, &expected, &theirs), Ok(()), "{name}");
                assert_eq!(explored.end, expected.end, "{name}");
            }
        }
    }
}

// The GPU explores every schedule of plain events from the net's tables and closes with the plain
// engine's configurations, events, end configurations and cycles.
#[test]
fn agreement() {
    let Some(engine) = Engine::new().unwrap() else {
        return;
    };
    let limit = Limit {
        record: usize::MAX,
        ..Limit::default()
    };
    let family = [
        family::dial(3),
        family::dial(5),
        family::diner(3),
        family::diner(5),
    ];
    for source in family.iter().map(String::as_str).chain([
        "A, [A] B",
        "A, A, [A] B",
        "A.B, A.C, [A, B] X, [A, C] Y",
        "A.A.A.A, [A.A] B",
        "A, K, [A] B, [[A] B] C, [C, K] D",
        "B, [B] C, [C] B",
        "[A.A] Z, B, [B] C, [C] B, (A, [Q] R)",
        "Seed.A.X, [Seed] (([A] B), ([A] C))",
        "A.X, [A] (B, C), [X, X] X, [X.B.C] Done",
        "Go.Y, K, [K] L, [Go] (X, [X] ().([Y] Z))",
        "A, Key, [A] B, [B, Key] (C, [Q] R)",
        "Case.(([P] True), ([P] False)), [Claim] Done, Claim",
        "Go, Go, [Go] (X, [X] Y)",
        "Go.A, Go.B, [Go] (X, [X] Y), [Y.A] Z, [Y.B, Z] W",
        "Go, Go, [Go] (X, [Q] R), [X] Y, [X] W, [Y, W] Done",
        "X.([A, C] B).A, C, C",
        "A, A, A, [A, A, A] B",
        "A.B, A.B, A, [A, A.B] C",
        "Claim, [Claim] P.Work, [Work] Done, [P] X",
    ]) {
        let program = frontend::lowering::parse(source).unwrap();
        let mut plain = Laser::plain(&program);
        plain.run(100_000_000, limit);
        let mut net = Net::new(&program).unwrap();
        let explored = engine
            .explore(&mut net, usize::MAX, limit, Cycle::Find)
            .unwrap();
        assert_eq!(explored.mirrors(&net, &plain), Ok(()), "{source}");
    }
}

// Limits stop the GPU as they stop the plain engine: a refused application leaves the exploration
// open, and the configuration limit ends it.
#[test]
fn limit() {
    let Some(engine) = Engine::new().unwrap() else {
        return;
    };
    let program = frontend::lowering::parse("A, [A] A.A").unwrap();
    let mut net = Net::new(&program).unwrap();
    let explored = engine
        .explore(&mut net, usize::MAX, Limit::default(), Cycle::Ignore)
        .unwrap();
    assert!(!explored.closed);
    let program = frontend::lowering::parse(&family::dial(6)).unwrap();
    let mut net = Net::new(&program).unwrap();
    let small = Limit {
        configuration: 100,
        ..Limit::default()
    };
    let explored = engine
        .explore(&mut net, usize::MAX, small, Cycle::Ignore)
        .unwrap();
    assert!(!explored.closed);
    assert_eq!(explored.configuration, 100);
}

// Explorations that follow one another reuse the device's memory, so every kind an entry leads to
// must be sized before a kernel reads it; repeated runs of a family whose parts are grounded ahead
// agree every time.
#[test]
fn repeat() {
    let Some(engine) = Engine::new().unwrap() else {
        return;
    };
    let limit = Limit {
        record: usize::MAX,
        configuration: usize::MAX,
        ..Limit::default()
    };
    let program = frontend::lowering::parse(&family::dial(8)).unwrap();
    let mut plain = Laser::plain(&program);
    plain.run(usize::MAX, limit);
    for _ in 0..4 {
        let mut net = Net::new(&program).unwrap();
        let explored = engine
            .explore(&mut net, usize::MAX, limit, Cycle::Find)
            .unwrap();
        assert_eq!(explored.mirrors(&net, &plain), Ok(()));
    }
}
