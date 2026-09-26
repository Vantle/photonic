use crate::handle::Handle;
use serde::{Serialize, Serializer};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Atom(String),
    Rule(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Occurrence {
    pub id: usize,
    pub value: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Coherence {
    pub frame: usize,
    pub occurrence: Vec<Occurrence>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Opener {
    Program,
    Rule(usize),
}

impl Serialize for Opener {
    fn serialize<Sink: Serializer>(&self, serializer: Sink) -> Result<Sink::Ok, Sink::Error> {
        match self {
            Self::Program => serializer.serialize_str("program"),
            Self::Rule(rule) => serializer.collect_str(&Handle::Rule(*rule)),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub opener: Option<Opener>,
    pub parent: Option<usize>,
    pub lexical: Option<usize>,
    pub rule: Vec<Occurrence>,
    pub held: Vec<Occurrence>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Configuration {
    pub coherence: Vec<Coherence>,
    pub frame: Vec<Frame>,
    pub supported: bool,
}
