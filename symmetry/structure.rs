use crate::graph::Graph;
use crate::group::{Permutation, Size};
use crate::measure;
use crate::rename;
use crate::search::{Exhausted, search};
use crate::twin::reduce;
use code::atom::Atom;
use code::configuration::Configuration;
use code::program::Program;
use hashing::combine;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Part {
    pub program: Program,
    pub configuration: Configuration,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Structure {
    pub program: Part,
    pub target: Option<Part>,
    pub pin: Vec<Atom>,
}

#[derive(Clone, Debug)]
pub struct Symmetry {
    pub atom: Vec<Atom>,
    pub(crate) key: Vec<u64>,
    pub block: Vec<Vec<Atom>>,
    pub generator: Vec<Permutation>,
    pub size: Size,
    pub node: usize,
    pub form: Structure,
}

fn factorial(size: usize) -> impl Iterator<Item = u64> {
    (2..=size as u64).rev()
}

pub(crate) fn image(part: &Part, map: &impl Fn(Atom) -> Atom) -> Part {
    Part {
        program: rename::program(&part.program, map),
        configuration: rename::configuration(&part.configuration, map),
    }
}

impl Structure {
    pub(crate) fn part(&self) -> impl Iterator<Item = (u32, &Part)> {
        std::iter::once((0, &self.program)).chain(self.target.iter().map(|target| (1, target)))
    }

    pub fn atom(&self) -> Vec<Atom> {
        let mut atom = BTreeSet::new();
        for (_, part) in self.part() {
            for entry in part.program.rule() {
                measure::rule(entry, &mut atom);
            }
            for entry in part.configuration.coherence() {
                measure::particle(entry, &mut atom);
            }
        }
        atom.into_iter().collect()
    }

    pub fn symmetry(&self, budget: usize) -> Result<Symmetry, Exhausted> {
        let atom = self.atom();
        let graph = Graph::new(&atom, &self.pin, self.part());
        let quotient = reduce(&graph);
        let labeling = search(&quotient.graph, budget)?;
        let member = |class: u32| {
            quotient.class[class as usize]
                .iter()
                .map(|&vertex| atom[vertex as usize])
        };
        let order = labeling
            .element
            .iter()
            .filter(|&&vertex| (vertex as usize) < quotient.class.len())
            .flat_map(|&class| member(class))
            .collect::<Vec<_>>();
        let generator = labeling
            .generator
            .iter()
            .map(|map| {
                Permutation::new(
                    (0..quotient.class.len() as u32)
                        .flat_map(|class| member(class).zip(member(map[class as usize]))),
                )
            })
            .filter(|permutation| !permutation.identity())
            .collect();
        let block = quotient
            .class
            .iter()
            .filter(|class| class.len() > 1)
            .map(|class| {
                class
                    .iter()
                    .map(|&vertex| atom[vertex as usize])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut factor = labeling.orbit;
        factor.extend(block.iter().flat_map(|class| factorial(class.len())));
        let canonical = canonical(&order);
        let form = Self {
            program: image(&self.program, &|atom| canonical[&atom]),
            target: self
                .target
                .as_ref()
                .map(|target| image(target, &|atom| canonical[&atom])),
            pin: self
                .pin
                .iter()
                .filter_map(|atom| canonical.get(atom).copied())
                .collect(),
        };
        Ok(Symmetry {
            atom: order,
            key: labeling.certificate,
            block,
            generator,
            size: Size::new(factor),
            node: labeling.node,
            form,
        })
    }
}

fn canonical(order: &[Atom]) -> BTreeMap<Atom, Atom> {
    order
        .iter()
        .enumerate()
        .map(|(position, &atom)| (atom, Atom(position as u16)))
        .collect()
}

impl Symmetry {
    pub fn fingerprint(&self) -> u64 {
        self.key.iter().copied().fold(0, combine)
    }

    pub(crate) fn canonical(&self) -> BTreeMap<Atom, Atom> {
        canonical(&self.atom)
    }

    pub(crate) fn isomorphism(&self, other: &Self) -> Option<BTreeMap<Atom, Atom>> {
        if self.key != other.key {
            return None;
        }
        let mut map = self
            .atom
            .iter()
            .copied()
            .zip(other.atom.iter().copied())
            .collect::<BTreeMap<_, _>>();
        for block in &self.block {
            let image = block.iter().map(|atom| map[atom]).collect::<BTreeSet<_>>();
            let (kept, moved): (Vec<Atom>, Vec<Atom>) =
                block.iter().partition(|atom| image.contains(atom));
            let free = image.iter().filter(|atom| !block.contains(atom));
            for &atom in &kept {
                map.insert(atom, atom);
            }
            for (&atom, &target) in moved.iter().zip(free) {
                map.insert(atom, target);
            }
        }
        Some(map)
    }
}
