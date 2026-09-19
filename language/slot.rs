#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Slot {
    pub world: usize,
    pub token: Vec<usize>,
    pub position: usize,
}
