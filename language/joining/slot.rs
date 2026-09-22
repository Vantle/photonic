#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct Slot {
    pub site: usize,
    pub token: Vec<usize>,
    pub position: usize,
}
