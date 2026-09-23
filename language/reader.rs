#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Read {
    World(usize, usize),
    Context(usize, usize),
}

impl Read {
    pub fn place(self, index: &crate::index::Index) -> crate::place::Place {
        match self {
            Self::World(site, resource) => crate::place::Place::World(index.world(site), resource),
            Self::Context(frame, resource) => crate::place::Place::Context(frame, resource),
        }
    }
}

pub(crate) struct Reader {
    pub rule: usize,
    pub site: usize,
    pub resource: usize,
    pub owner: usize,
}

impl Reader {
    pub fn read(&self) -> Read {
        Read::World(self.site, self.resource)
    }
}
