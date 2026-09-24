use crate::particle::Particle;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(from = "Vec<Particle>", into = "Vec<Particle>")]
pub struct Configuration {
    coherence: Vec<Particle>,
}

impl From<Vec<Particle>> for Configuration {
    fn from(mut coherence: Vec<Particle>) -> Self {
        coherence.sort_unstable();
        Self { coherence }
    }
}

impl From<Configuration> for Vec<Particle> {
    fn from(configuration: Configuration) -> Self {
        configuration.coherence
    }
}

impl Configuration {
    pub fn coherence(&self) -> &[Particle] {
        &self.coherence
    }
}
