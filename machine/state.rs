use crate::coherence::{Coherence, Token};
use code::atom::Atom;
use code::canonical::{Key, key};
use code::configuration::Configuration;
use code::observation::{Observation, Occurrence};
use code::value::Value;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct State {
    coherence: Vec<Coherence>,
    next: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Membership {
    pub id: u32,
    pub coherence: Vec<usize>,
}

impl State {
    pub fn new(configuration: &Configuration) -> Option<Self> {
        let mut next = 0;
        let coherence = configuration
            .coherence()
            .iter()
            .map(|particle| {
                Some(Coherence::new(
                    particle
                        .flat()?
                        .into_iter()
                        .map(|atom| {
                            next += 1;
                            Token { atom, id: next - 1 }
                        })
                        .collect(),
                ))
            })
            .collect::<Option<_>>()?;
        Some(Self { coherence, next })
    }

    pub fn assemble(coherence: Vec<Coherence>, next: u32) -> Self {
        Self { coherence, next }
    }

    pub fn coherence(&self) -> &[Coherence] {
        &self.coherence
    }

    pub fn next(&self) -> u32 {
        self.next
    }

    pub fn size(&self) -> usize {
        self.coherence.iter().map(Coherence::len).sum()
    }

    pub fn configuration(&self) -> Configuration {
        Configuration::from(
            self.coherence
                .iter()
                .map(Coherence::particle)
                .collect::<Vec<_>>(),
        )
    }

    pub fn observation(&self) -> Observation {
        Observation::new(
            self.coherence
                .iter()
                .map(|coherence| {
                    coherence
                        .token()
                        .iter()
                        .map(|token| Occurrence {
                            id: token.id,
                            value: Value::Atom(token.atom),
                        })
                        .collect()
                })
                .collect(),
        )
    }

    pub fn key(&self, budget: usize) -> Key<Atom> {
        key(
            &self
                .coherence
                .iter()
                .map(|coherence| {
                    coherence
                        .token()
                        .iter()
                        .map(|token| (token.id, token.atom))
                        .collect()
                })
                .collect::<Vec<Vec<_>>>(),
            budget,
        )
    }

    pub fn membership(&self) -> Vec<Membership> {
        let mut pair = self
            .coherence
            .iter()
            .enumerate()
            .flat_map(|(index, coherence)| {
                coherence.token().iter().map(move |token| (token.id, index))
            })
            .collect::<Vec<_>>();
        pair.sort_unstable();
        pair.chunk_by(|left, right| left.0 == right.0)
            .filter(|chunk| chunk.len() > 1)
            .map(|chunk| Membership {
                id: chunk[0].0,
                coherence: chunk.iter().map(|&(_, index)| index).collect(),
            })
            .collect()
    }
}
