use crate::context;
use crate::occurrence::Occurrence;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Identity(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct World {
    pub identity: Identity,
    pub context: context::Identity,
    pub occurrence: Vec<Occurrence>,
}
