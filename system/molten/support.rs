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

struct Premise {
    head: Atom,
    positive: Vec<Atom>,
    negative: Vec<Atom>,
    certain: bool,
    possible: bool,
}

fn component(clause: &[Clause]) -> Vec<Vec<Atom>> {
    let atom = clause
        .iter()
        .flat_map(|clause| {
            std::iter::once(clause.head)
                .chain(clause.positive.iter().copied())
                .chain(clause.negative.iter().copied())
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let index = atom
        .iter()
        .enumerate()
        .map(|(index, &atom)| (atom, index))
        .collect::<HashMap<_, _>>();
    let mut forward = vec![Vec::new(); atom.len()];
    let mut reverse = vec![Vec::new(); atom.len()];
    for clause in clause {
        let head = index[&clause.head];
        for dependency in clause.positive.union(&clause.negative) {
            let dependency = index[dependency];
            forward[head].push(dependency);
            reverse[dependency].push(head);
        }
    }
    let mut visited = vec![false; atom.len()];
    let mut finished = Vec::new();
    for start in 0..atom.len() {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut pending = vec![(start, 0)];
        while let Some((current, position)) = pending.last_mut() {
            if *position == forward[*current].len() {
                finished.push(*current);
                pending.pop();
                continue;
            }
            let next = forward[*current][*position];
            *position += 1;
            if !visited[next] {
                visited[next] = true;
                pending.push((next, 0));
            }
        }
    }
    visited.fill(false);
    let mut result = Vec::new();
    for start in finished.into_iter().rev() {
        if visited[start] {
            continue;
        }
        visited[start] = true;
        let mut pending = vec![start];
        let mut selected = Vec::new();
        while let Some(current) = pending.pop() {
            selected.push(atom[current]);
            for &next in &reverse[current] {
                if !visited[next] {
                    visited[next] = true;
                    pending.push(next);
                }
            }
        }
        result.push(selected);
    }
    result.reverse();
    result
}

fn consequence(clause: &[Premise], assumption: &HashSet<Atom>, certain: bool) -> HashSet<Atom> {
    let active = clause
        .iter()
        .filter(|clause| {
            (if certain {
                clause.certain
            } else {
                clause.possible
            }) && !clause.negative.iter().any(|atom| assumption.contains(atom))
        })
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
        let mut definition = HashMap::<Atom, Vec<&Clause>>::new();
        for clause in &clause {
            definition.entry(clause.head).or_default().push(clause);
        }
        let mut lower = HashSet::new();
        let mut upper = HashSet::new();
        for component in component(&clause) {
            let member = component.into_iter().collect::<HashSet<_>>();
            let local = member
                .iter()
                .flat_map(|atom| definition.get(atom).into_iter().flatten())
                .map(|clause| {
                    let positive = clause
                        .positive
                        .iter()
                        .filter(|atom| !member.contains(atom))
                        .copied()
                        .collect::<Vec<_>>();
                    let negative = clause
                        .negative
                        .iter()
                        .filter(|atom| !member.contains(atom))
                        .copied()
                        .collect::<Vec<_>>();
                    Premise {
                        head: clause.head,
                        positive: clause
                            .positive
                            .iter()
                            .filter(|atom| member.contains(atom))
                            .copied()
                            .collect(),
                        negative: clause
                            .negative
                            .iter()
                            .filter(|atom| member.contains(atom))
                            .copied()
                            .collect(),
                        certain: positive.iter().all(|atom| lower.contains(atom))
                            && negative.iter().all(|atom| !upper.contains(atom)),
                        possible: positive.iter().all(|atom| upper.contains(atom))
                            && negative.iter().all(|atom| !lower.contains(atom)),
                    }
                })
                .collect::<Vec<_>>();
            let mut certain = HashSet::new();
            let mut possible = member;
            loop {
                let next = consequence(&local, &possible, true);
                let bound = consequence(&local, &next, false);
                if next == certain && bound == possible {
                    break;
                }
                certain = next;
                possible = bound;
            }
            lower.extend(certain);
            upper.extend(possible);
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
