use crate::forest::Forest;
use crate::group::orbit;
use crate::pattern::{Pattern, discover};
use crate::search::Exhausted;
use crate::statement::Statement;
use crate::structure::{Structure, Symmetry};
use code::atom::Atom;
use std::cmp::Reverse;
use std::collections::BTreeSet;

pub struct Analysis {
    pub symmetry: Symmetry,
    pub orbit: Vec<Vec<Atom>>,
    pub statement: Vec<Vec<usize>>,
    pub pattern: Vec<Pattern>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Global,
    Local,
    Block,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Class {
    pub kind: Kind,
    pub part: Vec<Vec<Atom>>,
    pub statement: Vec<usize>,
}

impl Analysis {
    fn global(&self, statement: &[Statement]) -> Vec<Class> {
        let count = self.orbit.len();
        let mut forest = Forest::new(count + self.statement.len());
        for (index, member) in self.statement.iter().enumerate() {
            let atom = member
                .iter()
                .flat_map(|&position| statement[position].atom())
                .collect::<BTreeSet<_>>();
            for (orbit, entry) in self.orbit.iter().enumerate() {
                if entry.iter().any(|value| atom.contains(value)) {
                    forest.join(orbit, count + index);
                }
            }
        }
        let mut class = forest
            .group()
            .into_iter()
            .map(|group| {
                let mut class = Class {
                    kind: Kind::Global,
                    part: Vec::new(),
                    statement: Vec::new(),
                };
                for index in group {
                    match self.orbit.get(index) {
                        Some(orbit) => class.part.push(orbit.clone()),
                        None => class.statement.extend(&self.statement[index - count]),
                    }
                }
                class
            })
            .collect::<Vec<_>>();
        class.sort_by_key(|class| Reverse((class.statement.len(), class.part.len())));
        class
    }

    fn local(pattern: &Pattern) -> Option<Class> {
        let varying = pattern.varying();
        if varying.is_empty() {
            return None;
        }
        Some(Class {
            kind: Kind::Local,
            part: pattern
                .occurrence
                .iter()
                .map(|occurrence| {
                    varying
                        .iter()
                        .map(|&position| occurrence.atom[position])
                        .collect()
                })
                .collect(),
            statement: pattern
                .occurrence
                .iter()
                .flat_map(|occurrence| occurrence.statement.iter().copied())
                .collect(),
        })
    }

    pub fn class(&self, statement: &[Statement]) -> Vec<Class> {
        let block = self.symmetry.block.iter().map(|block| Class {
            kind: Kind::Block,
            part: vec![block.clone()],
            statement: Vec::new(),
        });
        let mut class = self.global(statement);
        class.extend(self.pattern.iter().filter_map(Self::local));
        class.extend(block);
        class
    }
}

pub fn analyze(
    whole: &Structure,
    statement: &[Statement],
    budget: usize,
) -> Result<Analysis, Exhausted> {
    let symmetry = whole.symmetry(budget)?;
    let atom = whole.atom();
    let atom = orbit(&atom, &symmetry.generator, |atom, permutation| {
        permutation.image(*atom)
    })
    .into_iter()
    .filter(|member| member.len() > 1)
    .map(|member| member.iter().map(|&index| atom[index]).collect())
    .collect();
    let moved = orbit(statement, &symmetry.generator, |entry, permutation| {
        entry.rename(|atom| permutation.image(atom))
    })
    .into_iter()
    .filter(|member| member.len() > 1)
    .collect::<Vec<_>>();
    let fixed = moved.iter().flatten().copied().collect::<BTreeSet<_>>();
    Ok(Analysis {
        pattern: discover(statement, &whole.pin, &fixed, budget)?,
        symmetry,
        orbit: atom,
        statement: moved,
    })
}
