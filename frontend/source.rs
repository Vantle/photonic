use serde::{Deserialize, Serialize};

use crate::failure::Failure;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    #[serde(default)]
    pub initial: Vec<Vec<Value>>,
    #[serde(default)]
    pub rule: Vec<Definition>,
}

impl Program {
    pub fn append(&mut self, program: Self) {
        self.initial.extend(program.initial);
        self.rule.extend(program.rule);
    }

    pub fn declare(&mut self, library: Self, name: impl std::fmt::Display) -> Result<(), Failure> {
        if !library.initial.is_empty() {
            return Err(Failure::Library {
                library: name.to_string(),
            });
        }
        self.rule.extend(library.rule);
        Ok(())
    }

    pub fn preserve(&mut self, program: &Self) {
        self.rule.extend(program.rule.iter().cloned());
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Value {
    Atom(String),
    Rule { rule: Box<Definition> },
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Definition {
    #[serde(default)]
    pub name: String,
    pub input: Vec<Vec<Value>>,
    pub output: Vec<Output>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    #[serde(default)]
    pub particle: Vec<Value>,
    #[serde(default)]
    pub body: Option<Vec<Definition>>,
}

impl Definition {
    pub fn canonical(&self) -> Self {
        let mut output = self
            .output
            .iter()
            .map(|value| Output {
                particle: particle(&value.particle),
                body: value.body.as_ref().map(|value| {
                    let mut result = value.iter().map(Self::canonical).collect::<Vec<_>>();
                    result.sort();
                    result
                }),
            })
            .collect::<Vec<_>>();
        output.sort();
        Self {
            name: String::new(),
            input: input(&self.input),
            output,
        }
    }
}

fn particle(value: &[Value]) -> Vec<Value> {
    let mut result = value
        .iter()
        .map(|value| match value {
            Value::Atom(atom) => Value::Atom(atom.clone()),
            Value::Rule { rule } => Value::Rule {
                rule: Box::new(rule.canonical()),
            },
        })
        .collect::<Vec<_>>();
    result.sort();
    result
}
fn input(value: &[Vec<Value>]) -> Vec<Vec<Value>> {
    let mut result = value
        .iter()
        .map(|value| particle(value))
        .collect::<Vec<_>>();
    result.sort();
    result
}
