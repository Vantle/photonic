use crate::rename;
use crate::structure::{Part, Structure};
use code::atom::Atom;
use code::configuration::Configuration;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::value::Value;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Statement {
    Rule(Rule),
    Coherence(Particle),
}

fn value(value: &Value, atom: &mut BTreeSet<Atom>) -> usize {
    match value {
        Value::Atom(entry) => {
            atom.insert(*entry);
            1
        }
        Value::Rule(nested) => rule(nested, atom),
    }
}

fn particle(particle: &Particle, atom: &mut BTreeSet<Atom>) -> usize {
    1 + particle
        .value()
        .iter()
        .map(|entry| value(entry, atom))
        .sum::<usize>()
}

fn rule(rule: &Rule, atom: &mut BTreeSet<Atom>) -> usize {
    let mut size = 1;
    for entry in rule.input() {
        size += particle(entry, atom);
    }
    for output in rule.output() {
        size += particle(output.particle(), atom);
        for nested in output.body().unwrap_or_default() {
            size += self::rule(nested, atom);
        }
    }
    size
}

impl Statement {
    fn measure(&self, atom: &mut BTreeSet<Atom>) -> usize {
        match self {
            Self::Rule(entry) => rule(entry, atom),
            Self::Coherence(entry) => particle(entry, atom),
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

    pub fn structure(&self) -> Structure {
        let (program, configuration) = match self {
            Self::Rule(entry) => (Program::from(vec![entry.clone()]), Configuration::default()),
            Self::Coherence(entry) => {
                (Program::default(), Configuration::from(vec![entry.clone()]))
            }
        };
        Structure {
            part: vec![Part {
                role: 0,
                program,
                configuration,
            }],
            pin: Vec::new(),
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
