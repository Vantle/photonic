use crate::measure;
use crate::rename;
use crate::structure::{Part, Structure};
use code::atom::Atom;
use code::configuration::Configuration;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::scope::Scope;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Statement {
    Rule(Rule),
    Coherence(Particle),
    Scope(Scope),
}

impl Statement {
    fn measure(&self, atom: &mut BTreeSet<Atom>) -> usize {
        match self {
            Self::Rule(entry) => measure::rule(entry, atom),
            Self::Coherence(entry) => measure::particle(entry, atom),
            Self::Scope(entry) => measure::scope(entry, atom),
        }
    }

    pub fn atom(&self) -> Vec<Atom> {
        let mut atom = BTreeSet::new();
        self.measure(&mut atom);
        atom.into_iter().collect()
    }

    pub(crate) fn size(&self) -> usize {
        self.measure(&mut BTreeSet::new())
    }

    pub(crate) fn rename(&self, map: impl Fn(Atom) -> Atom) -> Self {
        match self {
            Self::Rule(entry) => Self::Rule(rename::rule(entry, &map)),
            Self::Coherence(entry) => Self::Coherence(rename::particle(entry, &map)),
            Self::Scope(entry) => Self::Scope(rename::scope(entry, &map)),
        }
    }
}

pub fn structure(statement: &[Statement]) -> Structure {
    let mut rule = Vec::new();
    let mut coherence = Vec::new();
    let mut scope = Vec::new();
    for entry in statement {
        match entry {
            Statement::Rule(value) => rule.push(value.clone()),
            Statement::Coherence(value) => coherence.push(value.clone()),
            Statement::Scope(value) => scope.push(value.clone()),
        }
    }
    Structure {
        program: Part {
            program: Program::new(rule, scope),
            configuration: Configuration::from(coherence),
        },
        target: None,
        pin: Vec::new(),
    }
}
