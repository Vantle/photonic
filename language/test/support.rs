use crate::support::{Atom, Clause, Status, Support};
use std::collections::HashSet;

fn verify(clause: Vec<Clause>) {
    let support = Support::new(&clause);
    let mut incremental = Support::new(&[]);
    for (position, value) in clause.iter().enumerate() {
        incremental.insert(value);
        let reference = Support::new(&clause[..=position]);
        for value in &clause {
            assert_eq!(incremental.status(value.head), reference.status(value.head));
            for &atom in value.premise() {
                assert_eq!(incremental.status(atom), reference.status(atom));
            }
        }
    }
    let mut expected = HashSet::new();
    loop {
        let previous = expected.len();
        for clause in &clause {
            if clause.premise().iter().all(|atom| expected.contains(atom)) {
                expected.insert(clause.head);
            }
        }
        if previous == expected.len() {
            break;
        }
    }
    for atom in clause
        .iter()
        .flat_map(|clause| std::iter::once(clause.head).chain(clause.premise().iter().copied()))
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
            candidate.push(Clause::new(
                Atom::State(head),
                (0..3)
                    .filter(|index| premise & (1 << index) != 0)
                    .map(Atom::State),
            ));
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
    let rule = Clause::new(Atom::State(0), [Atom::State(0)]);
    assert_eq!(
        Support::new([&rule]).status(Atom::State(0)),
        Status::Unsupported
    );
    let fact = Clause::new(Atom::State(0), []);
    assert_eq!(
        Support::new([&rule, &fact, &fact]).status(Atom::State(0)),
        Status::Supported
    );
}

#[test]
fn chain() {
    let clause = (0..20_000)
        .map(|index| Clause::new(Atom::State(index), index.checked_sub(1).map(Atom::State)))
        .collect::<Vec<_>>();
    assert_eq!(
        Support::new(&clause).status(Atom::State(19_999)),
        Status::Supported
    );
}

#[test]
fn recycling() {
    let mut support = Support::new(&[]);
    let mut clause = Vec::new();
    for index in 0..256 {
        for value in [
            Clause::new(Atom::State(index * 2), [Atom::State(index * 2 + 1)]),
            Clause::new(Atom::State(index * 2 + 1), [Atom::State(index * 2)]),
            Clause::new(Atom::State(index * 2), [Atom::Event(index)]),
            Clause::new(Atom::Event(index), []),
        ] {
            support.insert(&value);
            clause.push(value);
            let expected = Support::new(&clause);
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
