use code::configuration::Configuration;
use code::observation::Observation;
use code::program::Program;
use serde::{Deserialize, Serialize};
use translation::failure::Failure;
use translation::vocabulary::Vocabulary;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Example {
    pub input: Configuration,
    pub output: Observation,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Goal {
    pub processor: f64,
    pub size: f64,
}

impl Default for Goal {
    fn default() -> Self {
        Self {
            processor: 4.0,
            size: 0.05,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub name: String,
    pub vocabulary: Vocabulary,
    pub example: Vec<Example>,
    pub holdout: Vec<Example>,
    pub reference: Option<Program>,
    pub goal: Option<Goal>,
}

const SPARE: [&str; 8] = [
    "Amber", "Basil", "Cider", "Dusk", "Ember", "Flint", "Grove", "Haze",
];

impl Task {
    pub fn conceal(self, count: usize) -> Result<Self, Failure> {
        let spare = SPARE
            .into_iter()
            .filter(|name| self.vocabulary.find(name).is_none())
            .take(count)
            .collect::<Vec<_>>();
        let mut vocabulary = self.vocabulary;
        for name in spare {
            vocabulary.intern(name)?;
        }
        Ok(Self { vocabulary, ..self })
    }
}
