#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Slot {
    pub location: crate::location::Location,
    pub token: Vec<usize>,
    pub position: usize,
}
