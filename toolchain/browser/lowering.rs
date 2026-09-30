use frontend::source::{self, Definition, Program};
use serde::Serialize;

// The book's lens titles each rule with its text, so the program it draws carries every rule's
// text beside its parts.
#[derive(Serialize)]
pub struct Lowered {
    initial: Vec<Vec<Value>>,
    rule: Vec<Rule>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    scope: Vec<Self>,
}

#[derive(Serialize)]
struct Rule {
    name: String,
    input: Vec<Vec<Value>>,
    output: Vec<Output>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum Value {
    Atom(String),
    Rule { rule: Rule },
}

#[derive(Serialize)]
#[serde(untagged)]
enum Output {
    Particle(Vec<Value>),
    Scope(Lowered),
}

fn particle(particle: &[source::Value]) -> Vec<Value> {
    particle
        .iter()
        .map(|value| match value {
            source::Value::Atom(atom) => Value::Atom(atom.clone()),
            source::Value::Rule { rule } => Value::Rule {
                rule: Rule::from(rule.as_ref()),
            },
        })
        .collect()
}

impl From<&Definition> for Rule {
    fn from(definition: &Definition) -> Self {
        Self {
            name: frontend::text::definition(definition),
            input: definition
                .input
                .iter()
                .map(|entry| particle(entry))
                .collect(),
            output: definition
                .output
                .iter()
                .map(|output| match output {
                    source::Output::Particle(entry) => Output::Particle(particle(entry)),
                    source::Output::Scope(program) => Output::Scope(Lowered::from(program)),
                })
                .collect(),
        }
    }
}

impl From<&Program> for Lowered {
    fn from(program: &Program) -> Self {
        Self {
            initial: program
                .initial
                .iter()
                .map(|entry| particle(entry))
                .collect(),
            rule: program.rule.iter().map(Rule::from).collect(),
            scope: program.scope.iter().map(Self::from).collect(),
        }
    }
}
