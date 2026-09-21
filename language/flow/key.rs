use super::Flow;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Weak};

pub(super) struct Key(Weak<Flow>);

impl Key {
    pub fn new(parent: &Arc<Flow>) -> Self {
        Self(Arc::downgrade(parent))
    }
    pub fn alive(&self) -> bool {
        self.0.strong_count() != 0
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.0.ptr_eq(&other.0)
    }
}

impl Eq for Key {}

impl Hash for Key {
    fn hash<State: Hasher>(&self, state: &mut State) {
        self.0.as_ptr().hash(state);
    }
}
