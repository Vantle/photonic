use crate::path::Path;
use crate::world;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proof {
    pub(crate) path: Path,
    pub(crate) world: world::Identity,
}

impl Proof {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn world(&self) -> world::Identity {
        self.world
    }
}
