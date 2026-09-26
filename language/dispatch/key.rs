#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Key {
    pub frame: usize,
    pub input: usize,
    pub owner: usize,
}
