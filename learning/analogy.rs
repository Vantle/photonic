use code::atom::Atom;
use code::rule::Rule;
use std::collections::BTreeMap;

const BUDGET: usize = 4_096;

pub struct Correspondence {
    pub atom: BTreeMap<Atom, Atom>,
    pub input: Vec<usize>,
    pub output: Vec<usize>,
}

struct Shape {
    input: Vec<Vec<Atom>>,
    output: Vec<Vec<Atom>>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Occurrence {
    output: bool,
    count: usize,
    length: usize,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Signature {
    occurrence: Vec<Occurrence>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Profile {
    input: Vec<usize>,
    output: Vec<usize>,
    signature: Vec<Signature>,
}

fn shape(rule: &Rule) -> Option<Shape> {
    if rule.output().iter().any(|output| output.body().is_some()) {
        return None;
    }
    Some(Shape {
        input: rule
            .input()
            .iter()
            .map(|particle| particle.flat())
            .collect::<Option<_>>()?,
        output: rule
            .output()
            .iter()
            .map(|output| output.particle().flat())
            .collect::<Option<_>>()?,
    })
}

fn length(list: &[Vec<Atom>]) -> Vec<usize> {
    let mut value = list.iter().map(Vec::len).collect::<Vec<_>>();
    value.sort_unstable();
    value
}

impl Shape {
    fn side(&self) -> [&[Vec<Atom>]; 2] {
        [&self.input, &self.output]
    }

    fn signature(&self) -> BTreeMap<Atom, Signature> {
        let mut result: BTreeMap<Atom, Signature> = BTreeMap::new();
        for (output, list) in [(false, &self.input), (true, &self.output)] {
            for particle in list {
                let mut count: BTreeMap<Atom, usize> = BTreeMap::new();
                for &atom in particle {
                    *count.entry(atom).or_default() += 1;
                }
                for (atom, count) in count {
                    result
                        .entry(atom)
                        .or_insert_with(|| Signature {
                            occurrence: Vec::new(),
                        })
                        .occurrence
                        .push(Occurrence {
                            output,
                            count,
                            length: particle.len(),
                        });
                }
            }
        }
        for entry in result.values_mut() {
            entry.occurrence.sort_unstable();
        }
        result
    }

    fn profile(&self) -> Profile {
        let mut signature = self.signature().into_values().collect::<Vec<_>>();
        signature.sort_unstable();
        Profile {
            input: length(&self.input),
            output: length(&self.output),
            signature,
        }
    }
}

fn image(particle: &[Atom], atom: &BTreeMap<Atom, Atom>) -> Vec<Atom> {
    let mut value = particle
        .iter()
        .filter_map(|entry| atom.get(entry).copied())
        .collect::<Vec<_>>();
    value.sort_unstable();
    value
}

fn within(part: &[Atom], whole: &[Atom]) -> bool {
    let mut rest = whole.iter();
    part.iter().all(|atom| rest.any(|entry| entry == atom))
}

fn pair(
    source: &[Vec<Atom>],
    target: &[Vec<Atom>],
    atom: &BTreeMap<Atom, Atom>,
) -> Option<Vec<usize>> {
    let mut used = vec![false; target.len()];
    source
        .iter()
        .map(|particle| {
            let mapped = image(particle, atom);
            let index = (0..target.len()).find(|&index| !used[index] && target[index] == mapped)?;
            used[index] = true;
            Some(index)
        })
        .collect()
}

struct Search<'shape> {
    source: &'shape Shape,
    target: &'shape Shape,
    order: Vec<(Atom, Vec<Atom>)>,
    member: BTreeMap<Atom, Vec<(usize, usize)>>,
    atom: BTreeMap<Atom, Atom>,
    taken: Vec<Atom>,
    used: [Vec<bool>; 2],
    node: usize,
}

enum Fit {
    Partial,
    Whole(usize),
}

impl Search<'_> {
    fn fit(&self, side: usize, index: usize) -> Option<Fit> {
        let particle = &self.source.side()[side][index];
        let mapped = image(particle, &self.atom);
        let host = self.target.side()[side];
        if mapped.len() < particle.len() {
            return host
                .iter()
                .any(|entry| entry.len() == particle.len() && within(&mapped, entry))
                .then_some(Fit::Partial);
        }
        (0..host.len())
            .find(|&slot| !self.used[side][slot] && host[slot] == mapped)
            .map(Fit::Whole)
    }

    fn place(&mut self, from: Atom) -> Option<Vec<(usize, usize)>> {
        let mut placed = Vec::new();
        for position in 0..self.member[&from].len() {
            let (side, index) = self.member[&from][position];
            match self.fit(side, index) {
                Some(Fit::Partial) => {}
                Some(Fit::Whole(slot)) => {
                    self.used[side][slot] = true;
                    placed.push((side, slot));
                }
                None => {
                    self.release(&placed);
                    return None;
                }
            }
        }
        Some(placed)
    }

    fn release(&mut self, placed: &[(usize, usize)]) {
        for &(side, slot) in placed {
            self.used[side][slot] = false;
        }
    }

    fn descend(&mut self, depth: usize) -> Option<Correspondence> {
        if self.node >= BUDGET {
            return None;
        }
        self.node += 1;
        let Some(&(from, _)) = self.order.get(depth) else {
            return Some(Correspondence {
                input: pair(&self.source.input, &self.target.input, &self.atom)?,
                output: pair(&self.source.output, &self.target.output, &self.atom)?,
                atom: self.atom.clone(),
            });
        };
        for choice in 0..self.order[depth].1.len() {
            let to = self.order[depth].1[choice];
            if self.taken.contains(&to) {
                continue;
            }
            self.atom.insert(from, to);
            self.taken.push(to);
            let found = self.place(from).and_then(|placed| {
                let found = self.descend(depth + 1);
                self.release(&placed);
                found
            });
            self.taken.pop();
            self.atom.remove(&from);
            if found.is_some() {
                return found;
            }
        }
        None
    }
}

pub fn correspond(source: &Rule, target: &Rule) -> Option<Correspondence> {
    let (source, target) = (shape(source)?, shape(target)?);
    if source.profile() != target.profile() {
        return None;
    }
    let destination = target.signature();
    let mut order = source
        .signature()
        .into_iter()
        .map(|(atom, signature)| {
            let candidate = destination
                .iter()
                .filter(|(_, value)| **value == signature)
                .map(|(&atom, _)| atom)
                .collect::<Vec<_>>();
            (atom, candidate)
        })
        .collect::<Vec<_>>();
    order.sort_by_key(|(_, candidate)| candidate.len());
    let mut member: BTreeMap<Atom, Vec<(usize, usize)>> = BTreeMap::new();
    for (side, list) in source.side().into_iter().enumerate() {
        for (index, particle) in list.iter().enumerate() {
            for &atom in particle {
                let entry = member.entry(atom).or_default();
                if !entry.contains(&(side, index)) {
                    entry.push((side, index));
                }
            }
        }
    }
    let used = [
        vec![false; target.input.len()],
        vec![false; target.output.len()],
    ];
    Search {
        source: &source,
        target: &target,
        order,
        member,
        atom: BTreeMap::new(),
        taken: Vec::new(),
        used,
        node: 0,
    }
    .descend(0)
}

pub fn partition(rule: &[&Rule]) -> Vec<Vec<usize>> {
    let mut bucket: BTreeMap<Profile, Vec<usize>> = BTreeMap::new();
    for (index, rule) in rule.iter().enumerate() {
        if let Some(shape) = shape(rule) {
            bucket.entry(shape.profile()).or_default().push(index);
        }
    }
    let mut result: Vec<Vec<usize>> = Vec::new();
    for member in bucket.into_values() {
        let mut group: Vec<Vec<usize>> = Vec::new();
        for index in member {
            match group
                .iter_mut()
                .find(|class| correspond(rule[class[0]], rule[index]).is_some())
            {
                Some(class) => class.push(index),
                None => group.push(vec![index]),
            }
        }
        result.extend(group);
    }
    result.sort_unstable();
    result
}
