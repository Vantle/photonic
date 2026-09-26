use serde::{Deserialize, Serialize};

use crate::failure::Failure;

// One level of source nesting adds at most five levels of JSON, and lowering refuses sources
// nested deeper than the parser's limit, so this admits every lowered program; serde_json's
// default of 128 would refuse programs nested about 32 levels deep.
const NESTING: usize = 8 * crate::parser::DEPTH;

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Program {
    #[serde(default)]
    pub initial: Vec<Vec<Value>>,
    #[serde(default)]
    pub rule: Vec<Definition>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scope: Vec<Self>,
}

impl Program {
    pub fn read(text: &str) -> Result<Self, serde_json::Error> {
        if nesting(text) > NESTING {
            return Err(serde::de::Error::custom(format!(
                "a program nests at most {NESTING} levels deep"
            )));
        }
        let mut deserializer = serde_json::Deserializer::from_str(text);
        deserializer.disable_recursion_limit();
        let program = Self::deserialize(&mut deserializer)?;
        deserializer.end()?;
        Ok(program)
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

    pub fn declare(&mut self, library: Self, name: impl std::fmt::Display) -> Result<(), Failure> {
        if !library.initial.is_empty() || !library.scope.is_empty() {
            return Err(Failure::Library {
                library: name.to_string(),
            });
        }
        self.rule.extend(library.rule);
        Ok(())
    }

    pub fn target(self) -> Result<Self, Failure> {
        if !self.scope.is_empty() {
            return Err(Failure::Target);
        }
        Ok(self)
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

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(untagged)]
pub enum Output {
    Particle(Vec<Value>),
    Scope(Program),
}

impl Definition {
    pub fn canonical(&self) -> Self {
        Self {
            name: String::new(),
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

fn nesting(text: &str) -> usize {
    let mut depth = 0_usize;
    let mut deepest = 0;
    let mut string = false;
    let mut escape = false;
    for byte in text.bytes() {
        if string {
            match (escape, byte) {
                (true, _) => escape = false,
                (false, b'\\') => escape = true,
                (false, b'"') => string = false,
                _ => {}
            }
            continue;
        }
        match byte {
            b'"' => string = true,
            b'[' | b'{' => {
                depth += 1;
                deepest = deepest.max(depth);
            }
            b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    deepest
}
