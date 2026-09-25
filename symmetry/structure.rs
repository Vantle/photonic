use crate::graph::{self, Graph};
use crate::group::{Permutation, Size};
use crate::rename;
use crate::search::{Exhausted, search};
use crate::twin::reduce;
use code::atom::Atom;
use code::configuration::Configuration;
use code::hashing::combine;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::value::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Part {
    pub role: u32,
    pub program: Program,
    pub configuration: Configuration,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Structure {
    pub part: Vec<Part>,
    pub pin: Vec<Atom>,
}

#[derive(Clone, Debug)]
pub struct Symmetry {
    pub atom: Vec<Atom>,
    pub key: Vec<u64>,
    pub block: Vec<Vec<Atom>>,
    pub generator: Vec<Permutation>,
    pub size: Size,
    pub node: usize,
}

fn value(value: &Value, atom: &mut BTreeSet<Atom>) {
    match value {
        Value::Atom(entry) => {
            atom.insert(*entry);
        }
        Value::Rule(nested) => rule(nested, atom),
    }
}

fn particle(particle: &Particle, atom: &mut BTreeSet<Atom>) {
    for entry in particle.value() {
        value(entry, atom);
    }
}

fn rule(rule: &Rule, atom: &mut BTreeSet<Atom>) {
    for entry in rule.input() {
        particle(entry, atom);
    }
    for output in rule.output() {
        particle(output.particle(), atom);
        for nested in output.body().unwrap_or_default() {
            self::rule(nested, atom);
        }
    }
}

fn factorial(size: usize) -> impl Iterator<Item = u64> {
    (2..=size as u64).rev()
}

impl Structure {
    pub fn atom(&self) -> Vec<Atom> {
        let mut atom = BTreeSet::new();
        for part in &self.part {
            for entry in part.program.rule() {
                rule(entry, &mut atom);
            }
            for entry in part.configuration.coherence() {
                particle(entry, &mut atom);
            }
        }
        atom.into_iter().collect()
    }

    pub fn rename(&self, map: impl Fn(Atom) -> Atom) -> Self {
        Self {
            part: self
                .part
                .iter()
                .map(|part| Part {
                    role: part.role,
                    program: rename::program(&part.program, &map),
                    configuration: rename::configuration(&part.configuration, &map),
                })
                .collect(),
            pin: self.pin.iter().map(|&atom| map(atom)).collect(),
        }
    }

    pub fn symmetry(&self, budget: usize) -> Result<Symmetry, Exhausted> {
        let atom = self.atom();
        let part = self
            .part
            .iter()
            .map(|part| graph::Part {
                role: part.role,
                program: &part.program,
                configuration: &part.configuration,
            })
            .collect::<Vec<_>>();
        let graph = Graph::new(&atom, &self.pin, &part);
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
            .collect();
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
        Ok(Symmetry {
            atom: order,
            key: labeling.certificate,
            block,
            generator,
            size: Size::new(factor),
            node: labeling.node,
        })
    }
}

impl Symmetry {
    pub fn fingerprint(&self) -> u64 {
        self.key.iter().copied().fold(0, combine)
    }

    pub fn canonical(&self) -> BTreeMap<Atom, Atom> {
        self.atom
            .iter()
            .enumerate()
            .map(|(position, &atom)| (atom, Atom(position as u16)))
            .collect()
    }

    pub fn form(&self, structure: &Structure) -> Structure {
        let map = self.canonical();
        structure.rename(|atom| map.get(&atom).copied().unwrap_or(atom))
    }

    pub fn isomorphism(&self, other: &Self) -> Option<BTreeMap<Atom, Atom>> {
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
