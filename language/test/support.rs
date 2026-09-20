use crate::support::{Atom, Clause, Status, Support};
use std::collections::{BTreeSet, HashSet};

fn verify(clause: Vec<Clause>) {
    let support = Support::new(clause.clone());
    let mut incremental = Support::new([]);
    for (position, value) in clause.iter().enumerate() {
        incremental.insert(value);
        let reference = Support::new(clause[..=position].iter().cloned());
        for value in &clause {
            assert_eq!(incremental.status(value.head), reference.status(value.head));
            for &atom in &value.premise {
                assert_eq!(incremental.status(atom), reference.status(atom));
            }
        }
    }
    let mut expected = HashSet::new();
    loop {
        let previous = expected.len();
        for clause in &clause {
            if clause.premise.iter().all(|atom| expected.contains(atom)) {
                expected.insert(clause.head);
            }
        }
        if previous == expected.len() {
            break;
        }
    }
    for atom in clause
        .iter()
        .flat_map(|clause| std::iter::once(clause.head).chain(clause.premise.iter().copied()))
    {
        assert_eq!(
            support.status(atom),
            if expected.contains(&atom) {
                Status::Supported
            } else {
                Status::Unsupported
            }
        );
    }
    assert_eq!(support.status(Atom::State(usize::MAX)), Status::Unsupported);
}

#[test]
fn exhaustive() {
    let mut candidate = Vec::new();
    for head in 0..3 {
        for premise in 0..8 {
            candidate.push(Clause {
                head: Atom::State(head),
                premise: (0..3)
                    .filter(|index| premise & (1 << index) != 0)
                    .map(Atom::State)
                    .collect(),
            });
        }
    }
    verify(Vec::new());
    for first in &candidate {
        verify(vec![first.clone()]);
        for second in &candidate {
            verify(vec![first.clone(), second.clone()]);
            for third in &candidate {
                verify(vec![first.clone(), second.clone(), third.clone()]);
            }
        }
    }
}

#[test]
fn cycle() {
    let rule = Clause {
        head: Atom::State(0),
        premise: BTreeSet::from([Atom::State(0)]),
    };
    assert_eq!(
        Support::new([rule.clone()]).status(Atom::State(0)),
        Status::Unsupported
    );
    let fact = Clause {
        head: Atom::State(0),
        premise: BTreeSet::new(),
    };
    assert_eq!(
        Support::new([rule, fact.clone(), fact]).status(Atom::State(0)),
        Status::Supported
    );
}

#[test]
fn chain() {
    let clause = (0..20_000).map(|index| Clause {
        head: Atom::State(index),
        premise: index.checked_sub(1).map(Atom::State).into_iter().collect(),
    });
    assert_eq!(
        Support::new(clause).status(Atom::State(19_999)),
        Status::Supported
    );
}

#[test]
fn recycling() {
    let mut support = Support::new([]);
    let mut clause = Vec::new();
    for index in 0..256 {
        for value in [
            Clause {
                head: Atom::State(index * 2),
                premise: BTreeSet::from([Atom::State(index * 2 + 1)]),
            },
            Clause {
                head: Atom::State(index * 2 + 1),
                premise: BTreeSet::from([Atom::State(index * 2)]),
            },
            Clause {
                head: Atom::State(index * 2),
                premise: BTreeSet::from([Atom::Event(index)]),
            },
            Clause {
                head: Atom::Event(index),
                premise: BTreeSet::new(),
            },
        ] {
            support.insert(&value);
            clause.push(value);
            let expected = Support::new(clause.clone());
            for value in 0..=index {
                for atom in [
                    Atom::State(value * 2),
                    Atom::State(value * 2 + 1),
                    Atom::Event(value),
                ] {
                    assert_eq!(support.status(atom), expected.status(atom));
                }
            }
        }
    }
}
