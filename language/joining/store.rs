use super::key::Key;
use super::node::Node;
use crate::budget::Account;
use hashing::Builder;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub(crate) struct Store {
    pub(super) domain: crate::candidate::Store,
    pub(super) preparation: Arc<crate::preparation::Store>,
    pub(super) budget: Account,
    account: Account,
    node: Mutex<HashMap<Key, Arc<Node>, Builder>>,
}

impl Store {
    pub fn new(capacity: usize) -> Self {
        let account = Account::new(capacity);
        Self {
            domain: crate::candidate::Store::new(account.clone()),
            preparation: Arc::new(crate::preparation::Store::new(account.clone())),
            budget: account.share(),
            account,
            node: Mutex::new(HashMap::default()),
        }
    }

    pub(super) fn subscribe(&self, key: Key) -> Option<Arc<Node>> {
        let mut node = self.node.lock().unwrap();
        if let Some(node) = node.get(&key) {
            return Some(node.clone());
        }
        let reservation = self.account.reserve(key.retained()).or_else(|| {
            node.retain(|_, node| Arc::strong_count(node) > 1);
            self.account.reserve(key.retained())
        })?;
        let entry = Arc::new(Node::new(reservation));
        node.insert(key, entry.clone());
        Some(entry)
    }

    #[cfg(test)]
    pub fn available(&self, size: usize) -> bool {
        self.budget.reserve(size).is_some()
    }

    pub fn evict(&self) {
        self.node.lock().unwrap().clear();
        self.domain.evict();
        self.preparation.evict();
    }

    pub fn retained(&self) -> usize {
        self.account.retained()
    }
}
