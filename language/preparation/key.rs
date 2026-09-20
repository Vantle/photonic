use super::Request;
use crate::pattern::Pattern;
use crate::program::Symbol;
use crate::state::World;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Weak};

pub(super) struct Key {
    pattern: Weak<Pattern>,
    world: Weak<World>,
    owner: usize,
}

impl Key {
    pub fn new(request: &Request<'_>) -> Self {
        Self {
            pattern: Arc::downgrade(request.pattern),
            world: Arc::downgrade(request.world),
            owner: if request
                .pattern
                .group
                .iter()
                .any(|group| matches!(group.value, Symbol::Rule(_)))
            {
                request.owner
            } else {
                0
            },
        }
    }

    pub fn alive(&self) -> bool {
        self.pattern.strong_count() != 0 && self.world.strong_count() != 0
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner
            && self.pattern.ptr_eq(&other.pattern)
            && self.world.ptr_eq(&other.world)
    }
}

impl Eq for Key {}

impl Hash for Key {
    fn hash<State: Hasher>(&self, state: &mut State) {
        self.pattern.as_ptr().hash(state);
        self.world.as_ptr().hash(state);
        self.owner.hash(state);
    }
}
