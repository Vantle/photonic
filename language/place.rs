use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Place {
    World(usize, usize),
    Context(usize, usize),
    Held(usize, usize),
}
