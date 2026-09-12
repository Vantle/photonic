use molten::support::{Atom, Clause, Status, Support};
use std::collections::{BTreeSet, HashSet};

fn consequence(clause: &[Clause], assumption: &HashSet<Atom>) -> HashSet<Atom> {
    let mut result = HashSet::new();
    loop {
        let previous = result.len();
        for clause in clause {
            if clause.positive.iter().all(|atom| result.contains(atom))
                && clause
                    .negative
                    .iter()
                    .all(|atom| !assumption.contains(atom))
            {
                result.insert(clause.head);
            }
        }
        if previous == result.len() {
            return result;
        }
    }
}

fn verify(clause: Vec<Clause>, open: Vec<usize>) {
    let support = Support::new(clause.clone(), open.clone());
    let mut clause = clause;
    for query in open {
        clause.push(Clause {
            head: Atom::Query(query),
            positive: BTreeSet::from([Atom::Open(query)]),
            negative: BTreeSet::new(),
        });
        clause.push(Clause {
            head: Atom::Open(query),
            positive: BTreeSet::new(),
            negative: BTreeSet::from([Atom::Open(query)]),
        });
    }
    let atom = clause
        .iter()
        .flat_map(|clause| {
            std::iter::once(clause.head)
                .chain(clause.positive.iter().copied())
                .chain(clause.negative.iter().copied())
        })
        .collect::<HashSet<_>>();
    let mut lower = HashSet::new();
    let mut upper = atom.clone();
    loop {
        let next = consequence(&clause, &upper);
        let bound = consequence(&clause, &next);
        if next == lower && bound == upper {
            break;
        }
        lower = next;
        upper = bound;
    }
    for atom in atom {
        let expected = if lower.contains(&atom) {
            Status::Supported
        } else if upper.contains(&atom) {
            Status::Conditional
        } else {
            Status::Unsupported
        };
        assert_eq!(support.status(atom), expected, "{atom:?}: {clause:?}");
    }
    assert_eq!(support.status(Atom::State(usize::MAX)), Status::Unsupported);
}

#[test]
fn exhaustive() {
    let mut candidate = Vec::new();
    for head in 0..2 {
        for positive in 0..4 {
            for negative in 0..4 {
                candidate.push(Clause {
                    head: Atom::State(head),
                    positive: (0..2)
                        .filter(|index| positive & (1 << index) != 0)
                        .map(Atom::State)
                        .collect(),
                    negative: (0..2)
                        .filter(|index| negative & (1 << index) != 0)
                        .map(Atom::State)
                        .collect(),
                });
            }
        }
    }
    verify(Vec::new(), Vec::new());
    for first in &candidate {
        verify(vec![first.clone()], Vec::new());
        for second in &candidate {
            verify(vec![first.clone(), second.clone()], Vec::new());
            for third in &candidate {
                verify(
                    vec![first.clone(), second.clone(), third.clone()],
                    Vec::new(),
                );
            }
        }
    }
}

fn random(seed: &mut u64) -> u64 {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    *seed >> 32
}

#[test]
fn generated() {
    let mut seed = 917;
    for _ in 0..2000 {
        let count = random(&mut seed) as usize % 14;
        let mut clause = Vec::new();
        for _ in 0..count {
            let head = Atom::Query(random(&mut seed) as usize % 6);
            let positive = random(&mut seed);
            let negative = random(&mut seed);
            clause.push(Clause {
                head,
                positive: (0..6)
                    .filter(|index| positive & (1 << index) != 0)
                    .map(Atom::Query)
                    .collect(),
                negative: (0..6)
                    .filter(|index| negative & (1 << index) != 0)
                    .map(Atom::Query)
                    .collect(),
            });
        }
        let open = random(&mut seed);
        verify(
            clause,
            (0..6).filter(|index| open & (1 << index) != 0).collect(),
        );
    }
}

#[test]
fn dependency() {
    let clause = vec![
        Clause {
            head: Atom::State(0),
            positive: BTreeSet::new(),
            negative: BTreeSet::from([Atom::State(0)]),
        },
        Clause {
            head: Atom::State(1),
            positive: BTreeSet::from([Atom::State(0)]),
            negative: BTreeSet::new(),
        },
        Clause {
            head: Atom::State(2),
            positive: BTreeSet::new(),
            negative: BTreeSet::from([Atom::State(1)]),
        },
        Clause {
            head: Atom::State(3),
            positive: BTreeSet::new(),
            negative: BTreeSet::new(),
        },
        Clause {
            head: Atom::State(2),
            positive: BTreeSet::from([Atom::State(3)]),
            negative: BTreeSet::new(),
        },
    ];
    let support = Support::new(clause.clone(), []);
    assert_eq!(support.status(Atom::State(0)), Status::Conditional);
    assert_eq!(support.status(Atom::State(1)), Status::Conditional);
    assert_eq!(support.status(Atom::State(2)), Status::Supported);
    assert_eq!(support.status(Atom::State(3)), Status::Supported);
    verify(clause, Vec::new());
}

#[test]
fn chain() {
    let clause = (0..20000)
        .map(|index| Clause {
            head: Atom::State(index),
            positive: index.checked_sub(1).map(Atom::State).into_iter().collect(),
            negative: BTreeSet::new(),
        })
        .collect::<Vec<_>>();
    let support = Support::new(clause, []);
    assert_eq!(support.status(Atom::State(19999)), Status::Supported);
}
