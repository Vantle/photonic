use crate::support::{Atom, Clause, Status, Support};
use std::collections::{BTreeSet, HashSet};

fn verify(clause: Vec<Clause>) {
    let support = Support::new(clause.clone());
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
