#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Identity(pub u64);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub identity: Identity,
    pub parent: Option<Identity>,
    pub lexical: Option<Identity>,
    pub declaration: Vec<crate::structure::Rule>,
    pub held: Vec<crate::occurrence::Occurrence>,
}
