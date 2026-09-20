use crate::basis::Set;

pub(crate) struct Change {
    pub world: Set<usize>,
    pub insertion: std::ops::Range<usize>,
    pub frame: Vec<usize>,
}

impl Change {
    pub fn retained(&self) -> usize {
        self.world.len() + self.frame.len() + 2
    }
}
