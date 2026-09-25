use code::atom::Atom;
use code::forest::Forest;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Permutation {
    map: BTreeMap<Atom, Atom>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Size {
    factor: Vec<u64>,
}

impl Permutation {
    pub(crate) fn new(pair: impl IntoIterator<Item = (Atom, Atom)>) -> Self {
        Self {
            map: pair.into_iter().filter(|(from, to)| from != to).collect(),
        }
    }

    pub(crate) fn image(&self, atom: Atom) -> Atom {
        self.map.get(&atom).copied().unwrap_or(atom)
    }

    pub(crate) fn identity(&self) -> bool {
        self.map.is_empty()
    }

    pub(crate) fn support(&self) -> usize {
        self.map.len()
    }

    pub(crate) fn compose(&self, other: &Self) -> Self {
        let atom = self
            .map
            .keys()
            .chain(other.map.keys())
            .copied()
            .collect::<BTreeSet<_>>();
        Self::new(
            atom.into_iter()
                .map(|atom| (atom, other.image(self.image(atom)))),
        )
    }

    pub fn cycle(&self) -> Vec<Vec<Atom>> {
        let mut seen = BTreeSet::new();
        let mut result = Vec::new();
        for &atom in self.map.keys() {
            if !seen.insert(atom) {
                continue;
            }
            let mut cycle = vec![atom];
            let mut next = self.image(atom);
            while seen.insert(next) {
                cycle.push(next);
                next = self.image(next);
            }
            result.push(cycle);
        }
        result
    }

    pub(crate) fn rename(&self, map: impl Fn(Atom) -> Atom) -> Self {
        Self::new(self.map.iter().map(|(&from, &to)| (map(from), map(to))))
    }
}

fn prime(mut value: u64, result: &mut Vec<u64>) {
    let mut divisor = 2;
    while divisor * divisor <= value {
        while value.is_multiple_of(divisor) {
            result.push(divisor);
            value /= divisor;
        }
        divisor += 1;
    }
    if value > 1 {
        result.push(value);
    }
}

impl Size {
    pub fn new(factor: Vec<u64>) -> Self {
        let mut result = Vec::new();
        for value in factor {
            prime(value, &mut result);
        }
        result.sort_unstable();
        Self { factor: result }
    }
}

impl fmt::Display for Size {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        const BASE: u128 = 1_000_000_000;
        let mut limb = vec![1u128];
        for &factor in &self.factor {
            let mut carry = 0u128;
            for value in &mut limb {
                let product = *value * u128::from(factor) + carry;
                *value = product % BASE;
                carry = product / BASE;
            }
            while carry > 0 {
                limb.push(carry % BASE);
                carry /= BASE;
            }
        }
        let mut text = limb.last().map_or_else(String::new, u128::to_string);
        for value in limb.iter().rev().skip(1) {
            text.push_str(&format!("{value:09}"));
        }
        formatter.write_str(&text)
    }
}

pub fn element(generator: &[Permutation], limit: usize) -> Option<Vec<Permutation>> {
    let mut known = BTreeSet::from([Permutation::default()]);
    let mut frontier = vec![Permutation::default()];
    while let Some(current) = frontier.pop() {
        for generator in generator {
            let next = current.compose(generator);
            if known.contains(&next) {
                continue;
            }
            if known.len() == limit {
                return None;
            }
            known.insert(next.clone());
            frontier.push(next);
        }
    }
    let mut element = known
        .into_iter()
        .filter(|permutation| !permutation.identity())
        .collect::<Vec<_>>();
    element.sort_by_key(Permutation::support);
    Some(element)
}

pub(crate) fn orbit<Item: Clone + Ord>(
    item: &[Item],
    generator: &[Permutation],
    apply: impl Fn(&Item, &Permutation) -> Item,
) -> Vec<Vec<usize>> {
    let index = item
        .iter()
        .enumerate()
        .map(|(position, entry)| (entry.clone(), position))
        .collect::<BTreeMap<_, _>>();
    let mut forest = Forest::new(item.len());
    for permutation in generator {
        for (position, entry) in item.iter().enumerate() {
            let image = apply(entry, permutation);
            if image == *entry {
                continue;
            }
            if let Some(&image) = index.get(&image) {
                forest.join(position, image);
            }
        }
    }
    forest.group()
}
