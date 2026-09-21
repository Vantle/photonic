use super::cache::Request;
use crate::state::World;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Weak};

pub(super) struct Key<Value> {
    identity: Weak<Value>,
    world: Weak<World>,
    context: usize,
}

impl<Value> Key<Value> {
    pub fn new(request: &Request<'_, Value>) -> Self {
        Self {
            identity: Arc::downgrade(request.identity),
            world: Arc::downgrade(request.world),
            context: request.context,
        }
    }

    pub fn alive(&self) -> bool {
        self.identity.strong_count() != 0 && self.world.strong_count() != 0
    }
}

impl<Value> PartialEq for Key<Value> {
    fn eq(&self, other: &Self) -> bool {
        self.context == other.context
            && self.identity.ptr_eq(&other.identity)
            && self.world.ptr_eq(&other.world)
    }
}

impl<Value> Eq for Key<Value> {}

impl<Value> Hash for Key<Value> {
    fn hash<State: Hasher>(&self, state: &mut State) {
        self.identity.as_ptr().hash(state);
        self.world.as_ptr().hash(state);
        self.context.hash(state);
    }
}
