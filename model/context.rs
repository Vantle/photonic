#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Identity(pub u64);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Reference {
    Captured(Identity),
    Local(usize),
}

pub(crate) trait Capture: Ord {
    fn collect(&self, result: &mut std::collections::BTreeSet<Identity>);
}

impl Capture for Identity {
    fn collect(&self, result: &mut std::collections::BTreeSet<Identity>) {
        result.insert(*self);
    }
}

impl Capture for Reference {
    fn collect(&self, result: &mut std::collections::BTreeSet<Identity>) {
        if let Self::Captured(identity) = self {
            result.insert(*identity);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub identity: Identity,
    pub parent: Option<Identity>,
    pub lexical: Option<Identity>,
    pub declaration: Vec<crate::structure::Rule>,
    pub held: Vec<crate::occurrence::Occurrence>,
}
