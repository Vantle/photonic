use crate::hashing::Builder;
use serde::Serialize;
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Atom {
    State(usize),
    Event(usize),
    View(usize),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Clause {
    pub head: Atom,
    premise: SmallVec<[Atom; 2]>,
}

impl Clause {
    pub fn new(head: Atom, premise: impl IntoIterator<Item = Atom>) -> Self {
        let mut premise = premise.into_iter().collect::<SmallVec<[Atom; 2]>>();
        premise.sort_unstable();
        premise.dedup();
        Self { head, premise }
    }

    pub fn premise(&self) -> &[Atom] {
        &self.premise
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Supported,
    Unsupported,
}

struct Pending {
    head: Atom,
    remaining: usize,
}

struct Network {
    dependency: HashMap<Atom, Vec<usize>, Builder>,
    pending: crate::arena::Store<Pending>,
}

pub struct Support {
    established: HashSet<Atom, Builder>,
    network: Option<Box<Network>>,
}

impl Support {
    pub fn new<'clause>(clause: impl IntoIterator<Item = &'clause Clause>) -> Self {
        let mut support = Self {
            established: HashSet::default(),
            network: None,
        };
        for clause in clause {
            support.insert(clause);
        }
        support
    }

    pub(crate) fn insert(&mut self, clause: &Clause) {
        if self.established.contains(&clause.head) {
            return;
        }
        let missing = clause
            .premise
            .iter()
            .copied()
            .filter(|atom| !self.established.contains(atom))
            .collect::<smallvec::SmallVec<[_; 2]>>();
        if missing.is_empty() {
            self.establish(clause.head);
            return;
        }
        let network = self.network.get_or_insert_with(|| {
            Box::new(Network {
                dependency: HashMap::default(),
                pending: crate::arena::Store::new(),
            })
        });
        let pending = network.pending.insert(Pending {
            head: clause.head,
            remaining: missing.len(),
        });
        for atom in missing {
            network.dependency.entry(atom).or_default().push(pending);
        }
    }

    fn establish(&mut self, atom: Atom) {
        let mut pending: smallvec::SmallVec<[Atom; 2]> = smallvec::smallvec![atom];
        while let Some(atom) = pending.pop() {
            if !self.established.insert(atom) {
                continue;
            }
            let Some(network) = self.network.as_mut() else {
                continue;
            };
            for position in network.dependency.remove(&atom).into_iter().flatten() {
                let waiting = &mut network.pending[position];
                waiting.remaining -= 1;
                if waiting.remaining == 0 {
                    pending.push(network.pending.remove(position).head);
                }
            }
            if network.dependency.is_empty() {
                self.network = None;
            }
        }
    }

    pub fn status(&self, atom: Atom) -> Status {
        if self.established.contains(&atom) {
            Status::Supported
        } else {
            Status::Unsupported
        }
    }
}
