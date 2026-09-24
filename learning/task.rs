use code::configuration::Configuration;
use code::observation::Observation;
use code::program::Program;
use serde::{Deserialize, Serialize};
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
    pub hidden: usize,
    pub example: Vec<Example>,
    pub holdout: Vec<Example>,
    pub reference: Option<Program>,
    #[serde(default)]
    pub goal: Option<Goal>,
}

pub const HIDDEN: [&str; 8] = [
    "Amber", "Basil", "Cider", "Dusk", "Ember", "Flint", "Grove", "Haze",
];

impl Task {
    pub fn conceal(mut self, count: usize) -> Self {
        let mut added = 0;
        for name in HIDDEN {
            if added == count {
                break;
            }
            if self.vocabulary.find(name).is_none() {
                self.vocabulary.intern(name);
                added += 1;
            }
        }
        self.hidden = added;
        self
    }
}
