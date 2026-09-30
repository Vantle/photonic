use std::ops::Range;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Module,
    List,
    Term,
    Group,
    Rule,
    Concept,
}

// Nodes keep the order pest reports them in, each with the index just past its subtree, so a
// node's children follow it and no node needs a list of its own; a parsed source then costs a few
// words for each node instead of an allocation for each.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
    pub kind: Kind,
    pub span: Range<usize>,
    pub(crate) next: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tree<'source> {
    source: &'source str,
    node: Vec<Node>,
}

impl<'source> Tree<'source> {
    pub(crate) fn new(source: &'source str, node: Vec<Node>) -> Self {
        Self { source, node }
    }

    pub fn source(&self) -> &'source str {
        self.source
    }

    pub fn node(&self) -> &[Node] {
        &self.node
    }

    pub fn child(&self, index: usize) -> impl Iterator<Item = usize> {
        let end = self.node[index].next;
        std::iter::successors(
            Some(index + 1).filter(|&first| first < end),
            move |&child| Some(self.node[child].next).filter(|&next| next < end),
        )
    }
}
