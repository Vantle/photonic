use super::Constraint;
use super::store::{Decision, Identity, Store};
use std::sync::Arc;

pub(super) struct Sharing {
    store: Arc<Store>,
    prefix: Vec<Option<Identity>>,
}

impl Sharing {
    pub fn new(store: Arc<Store>) -> Self {
        Self {
            store,
            prefix: Vec::new(),
        }
    }

    pub fn select(
        &self,
        parent: Option<usize>,
        constraint: Constraint,
        evaluate: impl FnOnce() -> bool,
    ) -> Decision {
        let identity = match parent {
            None => None,
            Some(parent) => match self.prefix[parent] {
                Some(identity) => Some(identity),
                None => {
                    return Decision {
                        accepted: evaluate(),
                        identity: None,
                    };
                }
            },
        };
        self.store.select(identity, constraint, evaluate)
    }

    pub fn push(&mut self, identity: Option<Identity>) {
        self.prefix.push(identity);
    }

    pub fn retained(&self) -> usize {
        self.prefix.len()
    }
}
