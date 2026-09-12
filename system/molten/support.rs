use serde::Serialize;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Atom {
    State(usize),
    Event(usize),
    View(usize),
    Query(usize),
    Open(usize),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Clause {
    pub head: Atom,
    pub positive: BTreeSet<Atom>,
    pub negative: BTreeSet<Atom>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Supported,
    Conditional,
    Unsupported,
}

pub struct Support {
    lower: HashSet<Atom>,
    upper: HashSet<Atom>,
}

fn consequence(clause: &[Clause], assumption: &HashSet<Atom>) -> HashSet<Atom> {
    let active = clause
        .iter()
        .filter(|clause| !clause.negative.iter().any(|atom| assumption.contains(atom)))
        .collect::<Vec<_>>();
    let mut remaining = active
        .iter()
        .map(|clause| clause.positive.len())
        .collect::<Vec<_>>();
    let mut dependency = HashMap::<Atom, Vec<usize>>::new();
    let mut pending = VecDeque::new();
    let mut result = HashSet::new();
    for (index, clause) in active.iter().enumerate() {
        if clause.positive.is_empty() {
            pending.push_back(clause.head);
        }
        for &atom in &clause.positive {
            dependency.entry(atom).or_default().push(index);
        }
    }
    while let Some(atom) = pending.pop_front() {
        if !result.insert(atom) {
            continue;
        }
        if let Some(dependency) = dependency.get(&atom) {
            for &index in dependency {
                remaining[index] -= 1;
                if remaining[index] == 0 {
                    pending.push_back(active[index].head);
                }
            }
        }
    }
    result
}

impl Support {
    pub fn new(
        clause: impl IntoIterator<Item = Clause>,
        open: impl IntoIterator<Item = usize>,
    ) -> Self {
        let mut clause = clause.into_iter().collect::<Vec<_>>();
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
        let mut lower = HashSet::new();
        let mut upper = clause
            .iter()
            .flat_map(|clause| {
                std::iter::once(clause.head)
                    .chain(clause.positive.iter().copied())
                    .chain(clause.negative.iter().copied())
            })
            .collect::<HashSet<_>>();
        loop {
            let next = consequence(&clause, &upper);
            let bound = consequence(&clause, &next);
            if next == lower && bound == upper {
                break;
            }
            lower = next;
            upper = bound;
        }
        Self { lower, upper }
    }

    pub fn status(&self, atom: Atom) -> Status {
        if self.lower.contains(&atom) {
            Status::Supported
        } else if self.upper.contains(&atom) {
            Status::Conditional
        } else {
            Status::Unsupported
        }
    }
}
