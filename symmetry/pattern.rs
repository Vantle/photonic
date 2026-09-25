use crate::group::element;
use crate::search::Exhausted;
use crate::statement::{Statement, structure};
use crate::structure::Structure;
use code::atom::Atom;
use code::forest::Forest;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

const ELEMENT: usize = 4096;
const ATTEMPT: usize = 200_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Occurrence {
    pub statement: Vec<usize>,
    pub atom: Vec<Atom>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pattern {
    pub occurrence: Vec<Occurrence>,
}

impl Pattern {
    pub fn varying(&self) -> Vec<usize> {
        let first = &self.occurrence[0];
        (0..first.atom.len())
            .filter(|&position| {
                self.occurrence
                    .iter()
                    .any(|occurrence| occurrence.atom[position] != first.atom[position])
            })
            .collect()
    }
}

struct Shape {
    key: Vec<u64>,
    atom: Vec<Atom>,
    block: Vec<Vec<usize>>,
    group: Option<Vec<Vec<usize>>>,
    size: usize,
}

#[derive(Clone)]
struct Embedding {
    statement: Vec<usize>,
    atom: Vec<Atom>,
    position: BTreeMap<Atom, usize>,
}

struct Growth<'shape> {
    shape: &'shape [Shape],
    index: BTreeMap<Atom, Vec<usize>>,
    free: Vec<bool>,
    attempt: usize,
}

fn shape(statement: &Statement, pin: &[Atom], budget: usize) -> Result<Shape, Exhausted> {
    let symmetry = Structure {
        pin: pin.to_vec(),
        ..structure(std::slice::from_ref(statement))
    }
    .symmetry(budget)?;
    let position = symmetry
        .atom
        .iter()
        .enumerate()
        .map(|(position, &atom)| (atom, position))
        .collect::<BTreeMap<_, _>>();
    let block = symmetry
        .block
        .iter()
        .map(|block| {
            let mut member = block.iter().map(|atom| position[atom]).collect::<Vec<_>>();
            member.sort_unstable();
            member
        })
        .collect();
    let group = element(&symmetry.generator, ELEMENT).map(|element| {
        std::iter::once((0..symmetry.atom.len()).collect())
            .chain(element.iter().map(|permutation| {
                symmetry
                    .atom
                    .iter()
                    .map(|&atom| position[&permutation.image(atom)])
                    .collect()
            }))
            .collect()
    });
    Ok(Shape {
        key: symmetry.key,
        atom: symmetry.atom,
        block,
        group,
        size: statement.size(),
    })
}

impl Embedding {
    fn new(statement: usize, shape: &Shape) -> Self {
        Self {
            statement: vec![statement],
            atom: shape.atom.clone(),
            position: shape
                .atom
                .iter()
                .enumerate()
                .map(|(position, &atom)| (atom, position))
                .collect(),
        }
    }

    fn extend(&self, statement: usize, addition: &[(Atom, usize)]) -> Self {
        let mut next = self.clone();
        next.statement.push(statement);
        for &(atom, position) in addition {
            next.position.insert(atom, position);
            if next.atom.len() <= position {
                next.atom.resize(position + 1, atom);
            }
            next.atom[position] = atom;
        }
        next
    }
}

fn attach(
    source: &Shape,
    target: &Shape,
    place: &BTreeMap<Atom, (usize, bool)>,
    occurrence: &BTreeMap<Atom, usize>,
) -> Option<Vec<(Atom, usize)>> {
    let group = source.group.as_ref()?;
    let mut inside = vec![false; source.atom.len()];
    for &position in source.block.iter().flatten() {
        inside[position] = true;
    }
    'element: for map in group {
        let mut addition = Vec::new();
        for position in (0..source.atom.len()).filter(|&position| !inside[position]) {
            let image = target.atom[map[position]];
            match (place[&source.atom[position]], occurrence.get(&image)) {
                ((motif, false), Some(&known)) if known == motif => {}
                ((motif, true), None) => addition.push((image, motif)),
                _ => continue 'element,
            }
        }
        for block in &source.block {
            let image = block
                .iter()
                .map(|&position| target.atom[map[position]])
                .collect::<Vec<_>>();
            let mut need = block
                .iter()
                .map(|&position| place[&source.atom[position]])
                .filter(|&(_, new)| !new)
                .map(|(motif, _)| motif)
                .collect::<Vec<_>>();
            let mut have = image
                .iter()
                .filter_map(|atom| occurrence.get(atom).copied())
                .collect::<Vec<_>>();
            need.sort_unstable();
            have.sort_unstable();
            if need != have {
                continue 'element;
            }
            let fresh = block
                .iter()
                .map(|&position| place[&source.atom[position]])
                .filter(|&(_, new)| new)
                .map(|(motif, _)| motif);
            addition.extend(
                image
                    .iter()
                    .filter(|atom| !occurrence.contains_key(atom))
                    .copied()
                    .zip(fresh),
            );
        }
        return Some(addition);
    }
    None
}

fn component(copy: &[Embedding]) -> Vec<Vec<usize>> {
    let mut forest = Forest::new(copy.len());
    let mut owner = BTreeMap::new();
    for (index, embedding) in copy.iter().enumerate() {
        for &atom in &embedding.atom {
            let first = *owner.entry(atom).or_insert(index);
            forest.join(first, index);
        }
    }
    forest.group()
}

impl Growth<'_> {
    fn step(&mut self, copy: &[Embedding], statement: usize) -> Vec<Embedding> {
        let reference = &copy[0];
        let source = &self.shape[statement];
        let mut place = BTreeMap::new();
        let mut next = reference.atom.len();
        for &atom in &source.atom {
            let entry = match reference.position.get(&atom) {
                Some(&position) => (position, false),
                None => {
                    next += 1;
                    (next - 1, true)
                }
            };
            place.insert(atom, entry);
        }
        let addition = place
            .iter()
            .filter(|(_, (_, new))| *new)
            .map(|(&atom, &(position, _))| (atom, position))
            .collect::<Vec<_>>();
        let anchor = source
            .atom
            .iter()
            .find_map(|atom| place.get(atom).filter(|(_, new)| !new))
            .map(|&(position, _)| position);
        let mut taken = copy
            .iter()
            .flat_map(|copy| copy.statement.iter().copied())
            .collect::<BTreeSet<_>>();
        taken.insert(statement);
        let mut result = vec![reference.extend(statement, &addition)];
        for other in &copy[1..] {
            let Some(anchor) = anchor else {
                break;
            };
            let candidate = self
                .index
                .get(&other.atom[anchor])
                .cloned()
                .unwrap_or_default();
            for target in candidate {
                if !self.free[target]
                    || taken.contains(&target)
                    || self.shape[target].key != source.key
                {
                    continue;
                }
                if self.attempt == 0 {
                    return result;
                }
                self.attempt -= 1;
                if let Some(addition) = attach(source, &self.shape[target], &place, &other.position)
                {
                    taken.insert(target);
                    result.push(other.extend(target, &addition));
                    break;
                }
            }
        }
        result
    }

    fn grow(&mut self, seed: &[usize]) -> (usize, Vec<Embedding>) {
        let mut copy = seed
            .iter()
            .map(|&statement| Embedding::new(statement, &self.shape[statement]))
            .collect::<Vec<_>>();
        let mut size = self.shape[seed[0]].size;
        let mut score = (copy.len() - 1) * size;
        loop {
            let used = copy
                .iter()
                .flat_map(|copy| copy.statement.iter().copied())
                .collect::<BTreeSet<_>>();
            let candidate = copy[0]
                .atom
                .iter()
                .flat_map(|atom| self.index.get(atom).into_iter().flatten().copied())
                .filter(|statement| self.free[*statement] && !used.contains(statement))
                .collect::<BTreeSet<_>>();
            let mut best: Option<(usize, usize, Vec<Embedding>)> = None;
            for statement in candidate {
                let result = self.step(&copy, statement);
                if result.len() < 2 {
                    continue;
                }
                let value = (result.len() - 1) * (size + self.shape[statement].size);
                if best.as_ref().is_none_or(|(known, _, _)| value > *known) {
                    best = Some((value, statement, result));
                }
            }
            match best {
                Some((value, statement, result)) if value > score => {
                    size += self.shape[statement].size;
                    score = value;
                    copy = result;
                }
                _ => return (score, copy),
            }
        }
    }
}

pub(crate) fn discover(
    statement: &[Statement],
    pin: &[Atom],
    fixed: &BTreeSet<usize>,
    budget: usize,
) -> Result<Vec<Pattern>, Exhausted> {
    let shape = statement
        .iter()
        .map(|entry| self::shape(entry, pin, budget))
        .collect::<Result<Vec<_>, _>>()?;
    let mut index = BTreeMap::<Atom, Vec<usize>>::new();
    for (position, shape) in shape.iter().enumerate() {
        for &atom in &shape.atom {
            index.entry(atom).or_default().push(position);
        }
    }
    let mut growth = Growth {
        shape: &shape,
        index,
        free: (0..statement.len())
            .map(|position| !fixed.contains(&position))
            .collect(),
        attempt: ATTEMPT,
    };
    let mut class = BTreeMap::<&[u64], Vec<usize>>::new();
    for (position, shape) in shape.iter().enumerate() {
        if growth.free[position] {
            class.entry(&shape.key).or_default().push(position);
        }
    }
    let mut seed = class
        .into_values()
        .filter(|member| member.len() > 1)
        .collect::<Vec<_>>();
    let mut cache = Vec::with_capacity(seed.len());
    let mut heap = BinaryHeap::new();
    for (order, member) in seed.iter().enumerate() {
        let (score, copy) = growth.grow(member);
        cache.push(copy);
        heap.push((score, Reverse(order)));
    }
    let mut pattern = Vec::new();
    while let Some((_, Reverse(order))) = heap.pop() {
        let valid = cache[order]
            .iter()
            .flat_map(|copy| &copy.statement)
            .all(|&statement| growth.free[statement]);
        if !valid {
            let member = seed[order]
                .iter()
                .copied()
                .filter(|&statement| growth.free[statement])
                .collect::<Vec<_>>();
            if member.len() > 1 {
                let (score, copy) = growth.grow(&member);
                cache[order] = copy;
                heap.push((score, Reverse(order)));
            }
            continue;
        }
        let part = (cache[order][0].statement.len() == 1)
            .then(|| component(&cache[order]))
            .filter(|part| part.len() > 1);
        if let Some(part) = part {
            for member in part.into_iter().filter(|member| member.len() > 1) {
                let member = member
                    .into_iter()
                    .map(|index| cache[order][index].statement[0])
                    .collect::<Vec<_>>();
                let (score, copy) = growth.grow(&member);
                heap.push((score, Reverse(seed.len())));
                seed.push(member);
                cache.push(copy);
            }
            continue;
        }
        for &statement in cache[order].iter().flat_map(|copy| &copy.statement) {
            growth.free[statement] = false;
        }
        pattern.push(Pattern {
            occurrence: std::mem::take(&mut cache[order])
                .into_iter()
                .map(|copy| Occurrence {
                    statement: copy.statement,
                    atom: copy.atom,
                })
                .collect(),
        });
    }
    Ok(pattern)
}
