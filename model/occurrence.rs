use crate::history::History;
use crate::structure::Value;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Identity(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Occurrence {
    pub identity: Identity,
    pub value: Value,
    pub history: History,
}
