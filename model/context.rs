#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Identity(pub u64);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Reference<Context = Identity> {
    Captured(Context),
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

impl<Context: Capture> Capture for Reference<Context> {
    fn collect(&self, result: &mut std::collections::BTreeSet<Identity>) {
        if let Self::Captured(identity) = self {
            identity.collect(result);
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
