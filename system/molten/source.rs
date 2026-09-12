use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Program {
    #[serde(default)]
    pub initial: Vec<Vec<Value>>,
    #[serde(default)]
    pub rule: Vec<Definition>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(untagged)]
pub enum Value {
    Atom(String),
    Rule {
        rule: Box<Definition>,
    },
    Variable {
        variable: String,
    },
    Structure {
        structure: String,
        particle: Vec<Value>,
    },
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Definition {
    #[serde(default)]
    pub name: String,
    pub input: Vec<Vec<Value>>,
    pub output: Vec<Output>,
    #[serde(default)]
    pub negative: Option<Vec<Vec<Value>>>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct Output {
    #[serde(default)]
    pub particle: Vec<Value>,
    #[serde(default)]
    pub body: Option<Vec<Definition>>,
}

impl Definition {
    pub fn canonical(&self) -> Self {
        fn particle(value: &[Value]) -> Vec<Value> {
            let mut result = value
                .iter()
                .map(|value| match value {
                    Value::Atom(atom) => Value::Atom(atom.clone()),
                    Value::Variable { variable } => Value::Variable {
                        variable: variable.clone(),
                    },
                    Value::Structure {
                        structure,
                        particle: content,
                    } => Value::Structure {
                        structure: structure.clone(),
                        particle: particle(content),
                    },
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
            negative: self.negative.as_ref().map(|value| input(value)),
        }
    }
}
