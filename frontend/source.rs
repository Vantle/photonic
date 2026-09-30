use serde::Serialize;

use crate::failure::Failure;

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Library {
    pub rule: Vec<Definition>,
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Program {
    pub initial: Vec<Vec<Value>>,
    pub rule: Vec<Definition>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<Self>,
}

impl Library {
    pub fn read(text: &str) -> Result<Self, Failure> {
        crate::json::library(text)
    }
}

impl Program {
    pub fn read(text: &str) -> Result<Self, Failure> {
        crate::json::program(text)
    }

    pub fn canonical(&self) -> Self {
        Self {
            initial: input(&self.initial),
            rule: sorted(self.rule.iter().map(Definition::canonical).collect()),
            scope: sorted(self.scope.iter().map(Self::canonical).collect()),
        }
    }

    pub fn append(&mut self, program: Self) {
        self.initial.extend(program.initial);
        self.rule.extend(program.rule);
        self.scope.extend(program.scope);
    }

    pub fn declare(&mut self, library: Library) {
        self.rule.extend(library.rule);
    }

    pub fn preserve(&mut self, program: &Self) {
        self.rule.extend(program.rule.iter().cloned());
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(untagged)]
pub enum Value {
    Atom(String),
    Rule { rule: Box<Definition> },
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Definition {
    pub input: Vec<Vec<Value>>,
    pub output: Vec<Output>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(untagged)]
pub enum Output {
    Particle(Vec<Value>),
    Scope(Program),
}

impl Definition {
    pub fn canonical(&self) -> Self {
        Self {
            input: input(&self.input),
            output: sorted(
                self.output
                    .iter()
                    .map(|value| match value {
                        Output::Particle(value) => Output::Particle(particle(value)),
                        Output::Scope(program) => Output::Scope(program.canonical()),
                    })
                    .collect(),
            ),
        }
    }
}

fn particle(value: &[Value]) -> Vec<Value> {
    sorted(
        value
            .iter()
            .map(|value| match value {
                Value::Atom(atom) => Value::Atom(atom.clone()),
                Value::Rule { rule } => Value::Rule {
                    rule: Box::new(rule.canonical()),
                },
            })
            .collect(),
    )
}

fn input(value: &[Vec<Value>]) -> Vec<Vec<Value>> {
    sorted(value.iter().map(|value| particle(value)).collect())
}

fn sorted<Item: Ord>(mut value: Vec<Item>) -> Vec<Item> {
    value.sort();
    value
}
