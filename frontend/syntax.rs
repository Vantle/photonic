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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
    pub kind: Kind,
    pub span: Range<usize>,
    pub parent: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tree<'source> {
    source: &'source str,
    node: Vec<Node>,
    child: Vec<Vec<usize>>,
}

impl<'source> Tree<'source> {
    pub(crate) fn new(source: &'source str, node: Vec<Node>) -> Self {
        let mut child = vec![Vec::new(); node.len()];
        for (position, entry) in node.iter().enumerate() {
            if let Some(parent) = entry.parent {
                child[parent].push(position);
            }
        }
        Self {
            source,
            node,
            child,
        }
    }

    pub fn source(&self) -> &'source str {
        self.source
    }

    pub fn node(&self) -> &[Node] {
        &self.node
    }

    pub fn child(&self, index: usize) -> &[usize] {
        &self.child[index]
    }
}
