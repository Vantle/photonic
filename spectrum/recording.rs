use crate::budget::Budget;
use crate::subject::Subject;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[schemars(
    description = "exhaustive explores every future; path follows one run, firing at each step the first event within the limits that the scheduler finds, which can witness a target but never prove it unreachable."
)]
pub enum Mode {
    #[default]
    Exhaustive,
    Path,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[schemars(
    description = "The engine that explores every future: the interpreter, or laser, which carries matches back along events and names configurations by their components. Both close with the same configurations, events and answers; each numbers handles in its own order. Direct paths always follow the interpreter's scheduler."
)]
pub enum Engine {
    #[default]
    Interpreter,
    Laser,
}

impl std::str::FromStr for Engine {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "interpreter" => Ok(Self::Interpreter),
            "laser" => Ok(Self::Laser),
            _ => Err(format!(
                "{text} is not an engine; the engines are interpreter and laser"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[schemars(
    description = "How handles are numbered. shape: by the program's canonical form, so they survive reordering and renaming; text: by the sorted program, so they survive reordering; source: as written, for direct paths."
)]
pub(crate) enum Order {
    Shape,
    Text,
    Source,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(
    description = "What a question is about: a program to explore, or the key of an exploration an earlier answer returned."
)]
pub struct Recording {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program: Option<Subject>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "The key of an exploration an earlier answer returned, such as x91c7f661, instead of program."
    )]
    pub exploration: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<Mode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<Engine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub budget: Option<Budget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<Goal>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(
    description = "In path mode, the complete configuration the path stops at, as Prism reads it."
)]
pub struct Goal {
    pub configuration: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[schemars(description = "The goal also lists every loaded root rule.")]
    pub preserve: bool,
}
