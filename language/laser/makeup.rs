// A configuration named by its parts: the number of its root and the kinds of its components,
// sorted, so equal configurations have equal makeups.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Makeup {
    pub root: u32,
    pub kind: Vec<u32>,
}
