use code::configuration::Configuration;
use code::observation::Observation;
use code::program::Program;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;
use translation::vocabulary::Vocabulary;

#[derive(Debug, Error, PartialEq)]
pub enum Failure {
    #[error("the processor count must be a finite number above 0")]
    Processor,
    #[error("the size weight must be a finite number of at least 0")]
    Size,
    #[error(
        "'{name}' cannot name a task: use letters, digits, '.', '-' and '_', starting with a letter or digit"
    )]
    Name { name: String },
    #[error("no task is named {name}")]
    Missing { name: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Example {
    pub input: Configuration,
    pub output: Observation,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(try_from = "Form")]
pub struct Goal {
    processor: f64,
    size: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Form {
    processor: f64,
    size: f64,
}

impl TryFrom<Form> for Goal {
    type Error = Failure;

    fn try_from(form: Form) -> Result<Self, Failure> {
        Self::new(form.processor, form.size)
    }
}

impl Goal {
    pub fn new(processor: f64, size: f64) -> Result<Self, Failure> {
        if !processor.is_finite() || processor <= 0.0 {
            return Err(Failure::Processor);
        }
        if !size.is_finite() || size < 0.0 {
            return Err(Failure::Size);
        }
        Ok(Self { processor, size })
    }

    pub fn processor(&self) -> f64 {
        self.processor
    }

    pub fn size(&self) -> f64 {
        self.size
    }
}

impl Default for Goal {
    fn default() -> Self {
        Self {
            processor: 4.0,
            size: 0.05,
        }
    }
}

pub(crate) fn validate(name: &str) -> Result<(), Failure> {
    let valid = name
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "._-".contains(character));
    if !valid {
        return Err(Failure::Name {
            name: name.to_owned(),
        });
    }
    Ok(())
}

fn named<'de, Source: Deserializer<'de>>(source: Source) -> Result<String, Source::Error> {
    let name = String::deserialize(source)?;
    validate(&name).map_err(serde::de::Error::custom)?;
    Ok(name)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    #[serde(deserialize_with = "named")]
    pub name: String,
    pub vocabulary: Vocabulary,
    pub example: Vec<Example>,
    pub holdout: Vec<Example>,
    pub reference: Option<Program>,
    pub goal: Option<Goal>,
}

pub fn find<'pool>(pool: &'pool [Task], name: &str) -> Result<&'pool Task, Failure> {
    pool.iter()
        .find(|task| task.name == name)
        .ok_or_else(|| Failure::Missing {
            name: name.to_owned(),
        })
}

const SPARE: [&str; 8] = [
    "Amber", "Basil", "Cider", "Dusk", "Ember", "Flint", "Grove", "Haze",
];

impl Task {
    pub(crate) fn conceal(self, count: usize) -> Result<Self, translation::failure::Failure> {
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
