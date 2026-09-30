use super::{Bound, Stop};
use crate::laser::Laser;
use crate::laser::net::{Cycle, Net};
use crate::path::Search;
use crate::runtime::{Limit, Runtime};

fn parse(source: &str) -> frontend::source::Program {
    frontend::lowering::parse(source).unwrap()
}

// The reasons each exhaustive engine gives, in the order they give them, reduced to their kinds so
// engines that count blocked events differently compare alike.
fn kind(stop: &[Stop]) -> Vec<(&'static str, Option<Bound>)> {
    stop.iter()
        .map(|stop| match *stop {
            Stop::Reached => ("reached", None),
            Stop::Cycle => ("cycle", None),
            Stop::End => ("end", None),
            Stop::Work { .. } => ("work", None),
            Stop::Record { .. } => ("record", None),
            Stop::Limit { bound, .. } => ("limit", Some(bound)),
            Stop::Start { bound, .. } => ("start", Some(bound)),
        })
        .collect()
}

fn exhaustive(source: &str, budget: usize, limit: Limit) -> [Vec<Stop>; 3] {
    let program = parse(source);
    let mut runtime = Runtime::new(&program);
    runtime.run(budget, limit);
    let mut laser = Laser::new(&program);
    laser.run(budget, limit);
    let mut plain = Laser::plain(&program);
    plain.run(budget, limit);
    [runtime.stop(), laser.stop(), plain.stop()]
}

#[test]
fn text() {
    for (stop, text) in [
        (
            Stop::Work { budget: 2_000_000 },
            "the work budget (2000000) ran out",
        ),
        (
            Stop::Record { budget: 100_000 },
            "the record budget (100000) filled",
        ),
        (
            Stop::Limit {
                bound: Bound::Occurrence,
                value: 256,
                blocked: 3,
            },
            "the occurrence limit (256) blocked 3 events",
        ),
        (
            Stop::Limit {
                bound: Bound::Configuration,
                value: 4_096,
                blocked: 1,
            },
            "the configuration limit (4096) blocked 1 event",
        ),
        (
            Stop::Start {
                bound: Bound::Occurrence,
                value: 256,
                measure: 301,
            },
            "s0 holds 301 occurrences; the occurrence limit is 256",
        ),
    ] {
        assert_eq!(stop.to_string(), text);
    }
}

// An exploration that closes gives no reason, and one that stops names each budget that ran out and
// each limit that blocked events, whichever exhaustive engine explored it.
#[test]
fn exploration() {
    let open = Limit::default();
    for (source, budget, limit, expected) in [
        ("A, [A] B", 1_000, open, vec![]),
        (
            "A, [A] A.A",
            1_000_000,
            Limit {
                occurrence: 3,
                ..open
            },
            vec![("limit", Some(Bound::Occurrence))],
        ),
        (
            "A, [A] A.A",
            1_000_000,
            Limit {
                configuration: 2,
                ..open
            },
            vec![("limit", Some(Bound::Configuration))],
        ),
        (
            "Go, [Go] (X, [X] Go)",
            1_000_000,
            Limit { scope: 0, ..open },
            vec![("limit", Some(Bound::Scope))],
        ),
        (
            "A, A, [A] B",
            1_000_000,
            Limit {
                coherence: 1,
                ..open
            },
            vec![("start", Some(Bound::Coherence))],
        ),
        (
            "A.A.A.A.A, [A.A] A",
            1_000_000,
            Limit {
                occurrence: 3,
                ..open
            },
            vec![("start", Some(Bound::Occurrence))],
        ),
        (&crate::family::dial(3), 5, open, vec![("work", None)]),
    ] {
        for (engine, stop) in exhaustive(source, budget, limit).iter().enumerate() {
            assert_eq!(kind(stop), expected, "{source} engine {engine}");
        }
    }
    let [runtime, laser, _] = exhaustive(
        "A.A.A.A.A, [A.A] A",
        1_000_000,
        Limit {
            occurrence: 3,
            ..open
        },
    );
    let start = Stop::Start {
        bound: Bound::Occurrence,
        value: 3,
        measure: 5,
    };
    assert_eq!((runtime, laser), (vec![start], vec![start]));
}

// Live rules are bounded by the scopes that hold them, so they never count as occurrences, and the
// root frame is not a scope a run opened.
#[test]
fn weight() {
    let rule = (0..300)
        .map(|index| format!("[R{index}] S{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let source = format!("{rule}, A, [A] B");
    for stop in exhaustive(&source, 1_000_000, Limit::default()) {
        assert_eq!(stop, vec![], "300 rules");
    }
    for stop in exhaustive(
        "A, [A] B",
        1_000_000,
        Limit {
            scope: 0,
            ..Limit::default()
        },
    ) {
        assert_eq!(stop, vec![], "no scope");
    }
}

// A record budget stops Laser and the interpreter, and a direct path, with the record budget named.
#[test]
fn record() {
    let limit = Limit {
        record: 20,
        ..Limit::default()
    };
    let program = parse(&crate::family::dial(4));
    let mut laser = Laser::new(&program);
    laser.run(1_000_000, limit);
    assert_eq!(laser.stop(), vec![Stop::Record { budget: 20 }]);
    let mut runtime = Runtime::new(&program);
    runtime.run(1_000_000, limit);
    assert_eq!(runtime.stop(), vec![Stop::Record { budget: 20 }]);
}

// A direct path stops at its goal, back at a configuration it passed, where no event applies, where
// the limits block every event, or where a budget runs out.
#[test]
fn path() {
    let open = Limit::default();
    for (source, goal, budget, limit, expected) in [
        ("A, [A] B", Some("B"), 1_000, open, vec![Stop::Reached]),
        ("A, [A] B, [B] A", Some("C"), 1_000, open, vec![Stop::Cycle]),
        ("A, [A] B", Some("C"), 1_000, open, vec![Stop::End]),
        (
            "A, [A] A.A",
            None,
            1_000_000,
            Limit {
                occurrence: 3,
                ..open
            },
            vec![Stop::Limit {
                bound: Bound::Occurrence,
                value: 3,
                blocked: 3,
            }],
        ),
        (
            "A, [A] A.A",
            None,
            1_000_000,
            Limit {
                configuration: 2,
                ..open
            },
            vec![Stop::Limit {
                bound: Bound::Configuration,
                value: 2,
                blocked: 1,
            }],
        ),
        ("A, [A] A.A", None, 3, open, vec![Stop::Work { budget: 3 }]),
    ] {
        let program = parse(source);
        let target = goal.map(|goal| crate::test::target(&program, goal));
        let mut search = Search::new(program, target);
        search.run(budget, limit);
        assert_eq!(search.stop(), expected, "{source}");
        assert_eq!(search.summary().stop, expected, "{source}");
    }
}

// The net names the same reasons, counting the events each limit refused.
#[test]
fn net() {
    let open = Limit {
        record: usize::MAX,
        ..Limit::default()
    };
    for (source, budget, limit, expected) in [
        ("A, [A] B", usize::MAX, open, vec![]),
        (
            "A, [A] A.A",
            usize::MAX,
            Limit {
                occurrence: 3,
                ..open
            },
            vec![Stop::Limit {
                bound: Bound::Occurrence,
                value: 3,
                blocked: 3,
            }],
        ),
        (
            "A, A, A, [A] B",
            usize::MAX,
            Limit {
                configuration: 2,
                ..open
            },
            vec![Stop::Limit {
                bound: Bound::Configuration,
                value: 2,
                blocked: 2,
            }],
        ),
        ("A, [A] A.A", 1, open, vec![Stop::Work { budget: 1 }]),
    ] {
        let program = parse(source);
        let explored = Net::new(&program)
            .unwrap()
            .explore(budget, limit, Cycle::Find)
            .unwrap();
        assert_eq!(explored.stop, expected, "{source}");
        assert_eq!(explored.closed, expected.is_empty(), "{source}");
    }
}
