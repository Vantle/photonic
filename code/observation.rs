use crate::canonical::{Key, key, membership};
use crate::configuration::Configuration;
use crate::particle::Particle;
use crate::value::Value;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Occurrence {
    pub id: u32,
    pub value: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Observation {
    coherence: Vec<Vec<Occurrence>>,
}

impl From<&Configuration> for Observation {
    fn from(configuration: &Configuration) -> Self {
        let mut id = 0;
        Self {
            coherence: configuration
                .coherence()
                .iter()
                .map(|particle| {
                    particle
                        .value()
                        .iter()
                        .map(|value| {
                            id += 1;
                            Occurrence {
                                id: id - 1,
                                value: value.clone(),
                            }
                        })
                        .collect()
                })
                .collect(),
        }
    }
}

impl Observation {
    pub fn new(coherence: Vec<Vec<Occurrence>>) -> Self {
        Self { coherence }
    }

    pub fn coherence(&self) -> &[Vec<Occurrence>] {
        &self.coherence
    }

    fn pair(&self) -> Vec<Vec<(u32, Value)>> {
        self.coherence
            .iter()
            .map(|entry| {
                entry
                    .iter()
                    .map(|occurrence| (occurrence.id, occurrence.value.clone()))
                    .collect()
            })
            .collect()
    }

    pub fn shared(&self) -> bool {
        !membership(&self.pair()).is_empty()
    }

    pub fn key(&self, budget: usize) -> Key<Value> {
        key(&self.pair(), budget)
    }

    pub fn configuration(&self) -> Configuration {
        Configuration::from(
            self.coherence
                .iter()
                .map(|entry| {
                    Particle::from(
                        entry
                            .iter()
                            .map(|occurrence| occurrence.value.clone())
                            .collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>(),
        )
    }
}
