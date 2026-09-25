use crate::atom::Atom;
use crate::rule::Rule;
use std::collections::BTreeMap;

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
    let mut value = particle.iter().map(|entry| atom[entry]).collect::<Vec<_>>();
    value.sort_unstable();
    value
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

fn search(
    order: &[(Atom, Vec<Atom>)],
    atom: &mut BTreeMap<Atom, Atom>,
    taken: &mut Vec<Atom>,
    source: &Shape,
    target: &Shape,
) -> Option<Correspondence> {
    let Some(((from, candidate), rest)) = order.split_first() else {
        return Some(Correspondence {
            input: pair(&source.input, &target.input, atom)?,
            output: pair(&source.output, &target.output, atom)?,
            atom: atom.clone(),
        });
    };
    for &to in candidate {
        if taken.contains(&to) {
            continue;
        }
        atom.insert(*from, to);
        taken.push(to);
        let found = search(rest, atom, taken, source, target);
        taken.pop();
        atom.remove(from);
        if found.is_some() {
            return found;
        }
    }
    None
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
    search(
        &order,
        &mut BTreeMap::new(),
        &mut Vec::new(),
        &source,
        &target,
    )
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
