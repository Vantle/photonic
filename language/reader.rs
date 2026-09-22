#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Read {
    World(usize, usize),
    Context(usize, usize),
}

impl Read {
    pub fn place(self, index: &crate::index::Index) -> crate::flow::Place {
        match self {
            Self::World(site, resource) => crate::flow::Place::World(index.world(site), resource),
            Self::Context(frame, resource) => crate::flow::Place::Context(frame, resource),
        }
    }
}

pub(crate) struct Reader {
    pub rule: usize,
    pub read: Read,
    pub owner: usize,
}
