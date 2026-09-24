use crate::atom::Atom;
use crate::rule::Rule;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum Value {
    Atom(Atom),
    Rule(Box<Rule>),
}

impl Value {
    pub fn atom(&self) -> Option<Atom> {
        match self {
            Self::Atom(atom) => Some(*atom),
            Self::Rule(_) => None,
        }
    }
}
