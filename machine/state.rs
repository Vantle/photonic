use crate::coherence::{Coherence, Token};
use code::atom::Atom;
use code::canonical::{Exhausted, Key, key};
use code::configuration::Configuration;
use code::observation::{Observation, Occurrence};
use code::value::Value;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct State {
    coherence: Vec<Coherence>,
    next: u32,
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

    pub fn key(&self, budget: usize) -> Result<Key<Atom>, Exhausted> {
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

    pub fn membership(&self) -> Vec<u32> {
        let coherence = self
            .coherence
            .iter()
            .map(|coherence| {
                coherence
                    .token()
                    .iter()
                    .map(|token| (token.id, ()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        code::canonical::membership(&coherence)
            .into_iter()
            .map(|(id, _)| id)
            .collect()
    }
}
