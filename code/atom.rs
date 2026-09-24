use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(transparent)]
pub struct Atom(pub u16);

impl Atom {
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}
