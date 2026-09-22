#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Location {
    World(usize),
    Context(usize),
}

impl Location {
    pub fn world(self) -> Option<usize> {
        match self {
            Self::World(world) => Some(world),
            Self::Context(_) => None,
        }
    }

    pub fn frame(self, state: &crate::state::State) -> usize {
        match self {
            Self::World(world) => state.world[world].frame,
            Self::Context(frame) => frame,
        }
    }
}
