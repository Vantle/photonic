use crate::state::State;
use hashing::Builder;
use std::collections::HashMap;
use std::sync::{Arc, Weak};

pub(super) struct Request<'state> {
    pub target: usize,
    pub capture: usize,
    pub state: &'state State,
}

pub(super) struct Store {
    entry: HashMap<(usize, usize), Weak<State>, Builder>,
    capacity: usize,
}

impl Default for Store {
    fn default() -> Self {
        Self::new(4096)
    }
}

impl Store {
    fn new(capacity: usize) -> Self {
        Self {
            entry: HashMap::default(),
            capacity,
        }
    }

    pub fn resolve(&mut self, request: Request<'_>) -> Arc<State> {
        let key = (request.target, request.capture);
        if let Some(environment) = self.entry.get(&key).and_then(Weak::upgrade) {
            return environment;
        }
        let environment = Arc::new(request.state.environment(request.capture));
        if self.entry.len() == self.capacity {
            self.entry.retain(|_, value| value.strong_count() != 0);
        }
        if self.entry.len() < self.capacity {
            self.entry.insert(key, Arc::downgrade(&environment));
        }
        environment
    }
}

#[cfg(test)]
#[path = "../test/environment.rs"]
mod test;
