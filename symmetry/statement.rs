use crate::measure;
use crate::rename;
use crate::structure::{Part, Structure};
use code::atom::Atom;
use code::configuration::Configuration;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Statement {
    Rule(Rule),
    Coherence(Particle),
}

impl Statement {
    fn measure(&self, atom: &mut BTreeSet<Atom>) -> usize {
        match self {
            Self::Rule(entry) => measure::rule(entry, atom),
            Self::Coherence(entry) => measure::particle(entry, atom),
        }
    }

    pub fn atom(&self) -> Vec<Atom> {
        let mut atom = BTreeSet::new();
        self.measure(&mut atom);
        atom.into_iter().collect()
    }

    pub fn size(&self) -> usize {
        self.measure(&mut BTreeSet::new())
    }

    pub fn rename(&self, map: impl Fn(Atom) -> Atom) -> Self {
        match self {
            Self::Rule(entry) => Self::Rule(rename::rule(entry, &map)),
            Self::Coherence(entry) => Self::Coherence(rename::particle(entry, &map)),
        }
    }
}

pub fn structure(statement: &[Statement]) -> Structure {
    let rule = statement
        .iter()
        .filter_map(|entry| match entry {
            Statement::Rule(rule) => Some(rule.clone()),
            Statement::Coherence(_) => None,
        })
        .collect::<Vec<_>>();
    let coherence = statement
        .iter()
        .filter_map(|entry| match entry {
            Statement::Coherence(particle) => Some(particle.clone()),
            Statement::Rule(_) => None,
        })
        .collect::<Vec<_>>();
    Structure {
        part: vec![Part {
            role: 0,
            program: Program::from(rule),
            configuration: Configuration::from(coherence),
        }],
        pin: Vec::new(),
    }
}
