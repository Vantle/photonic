use crate::slot::Slot;

struct Node {
    parent: Option<usize>,
    slot: Slot,
}

#[derive(Default)]
pub(crate) struct Arena {
    node: Vec<Node>,
}

impl Arena {
    pub fn push(&mut self, parent: Option<usize>, slot: Slot) -> usize {
        let index = self.node.len();
        self.node.push(Node { parent, slot });
        index
    }

    pub fn iter(&self, parent: Option<usize>) -> impl Iterator<Item = &Slot> {
        std::iter::successors(parent, |&index| self.node[index].parent)
            .map(|index| &self.node[index].slot)
    }

    pub fn complete(&self, parent: Option<usize>, slot: Slot) -> Vec<Slot> {
        let mut value = Vec::with_capacity(slot.position + 1);
        value.push(slot);
        value.extend(self.iter(parent).cloned());
        value.reverse();
        value
    }
}
