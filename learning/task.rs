use crate::reach;
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
    #[error("{name} names an atom outside its vocabulary of {count}")]
    Atom { name: String, count: usize },
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
#[serde(try_from = "Draft")]
pub struct Task {
    pub name: String,
    pub vocabulary: Vocabulary,
    pub example: Vec<Example>,
    pub holdout: Vec<Example>,
    pub reference: Option<Program>,
    pub goal: Option<Goal>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Draft {
    #[serde(deserialize_with = "named")]
    name: String,
    vocabulary: Vocabulary,
    example: Vec<Example>,
    holdout: Vec<Example>,
    reference: Option<Program>,
    goal: Option<Goal>,
}

impl TryFrom<Draft> for Task {
    type Error = Failure;

    fn try_from(draft: Draft) -> Result<Self, Failure> {
        let count = draft.vocabulary.len();
        let reach = draft
            .example
            .iter()
            .chain(&draft.holdout)
            .flat_map(|example| {
                [
                    reach::configuration(&example.input),
                    reach::configuration(&example.output.configuration()),
                ]
            })
            .chain(draft.reference.iter().map(reach::program))
            .max()
            .unwrap_or(0);
        if reach > count {
            return Err(Failure::Atom {
                name: draft.name,
                count,
            });
        }
        Ok(Self {
            name: draft.name,
            vocabulary: draft.vocabulary,
            example: draft.example,
            holdout: draft.holdout,
            reference: draft.reference,
            goal: draft.goal,
        })
    }
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
