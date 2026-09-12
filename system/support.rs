use serde::Serialize;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Atom {
    State(usize),
    Event(usize),
    View(usize),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Clause {
    pub head: Atom,
    pub premise: BTreeSet<Atom>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Supported,
    Unsupported,
}

pub struct Support {
    established: HashSet<Atom>,
}

impl Support {
    pub fn new(clause: impl IntoIterator<Item = Clause>) -> Self {
        let clause = clause.into_iter().collect::<Vec<_>>();
        let mut remaining = clause
            .iter()
            .map(|clause| clause.premise.len())
            .collect::<Vec<_>>();
        let mut dependency = HashMap::<Atom, Vec<usize>>::new();
        let mut pending = VecDeque::new();
        for (index, clause) in clause.iter().enumerate() {
            if clause.premise.is_empty() {
                pending.push_back(clause.head);
            }
            for &atom in &clause.premise {
                dependency.entry(atom).or_default().push(index);
            }
        }
        let mut established = HashSet::new();
        while let Some(atom) = pending.pop_front() {
            if !established.insert(atom) {
                continue;
            }
            for &index in dependency.get(&atom).into_iter().flatten() {
                remaining[index] -= 1;
                if remaining[index] == 0 {
                    pending.push_back(clause[index].head);
                }
            }
        }
        Self { established }
    }

    pub fn status(&self, atom: Atom) -> Status {
        if self.established.contains(&atom) {
            Status::Supported
        } else {
            Status::Unsupported
        }
    }
}
