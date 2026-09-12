use std::ops::Range;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Module,
    Concept,
    Group,
    Context,
    Continuation,
    Coherence,
    Space,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
    pub kind: Kind,
    pub span: Range<usize>,
    pub parent: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tree<'source> {
    pub(crate) source: &'source str,
    pub(crate) node: Vec<Node>,
}

impl<'source> Tree<'source> {
    pub fn source(&self) -> &'source str {
        self.source
    }

    pub fn node(&self) -> &[Node] {
        &self.node
    }
}
