use super::key::Key;
use super::node::Node;
use crate::budget::{Account, Budget};
use crate::hashing::Builder;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub(crate) struct Store {
    pub domain: crate::candidate::Store,
    pub preparation: Arc<crate::preparation::Store>,
    account: Account,
    node: Mutex<HashMap<Key, Arc<Node>, Builder>>,
}

impl Store {
    pub fn new(capacity: usize) -> Self {
        let account = Account::new(capacity);
        Self {
            domain: crate::candidate::Store::new(account.clone()),
            preparation: Arc::new(crate::preparation::Store::new(account.clone())),
            account,
            node: Mutex::new(HashMap::default()),
        }
    }

    pub fn budget(&self) -> &Arc<Budget> {
        self.account.budget()
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

    pub fn evict(&self) {
        self.node.lock().unwrap().clear();
        self.domain.evict();
        self.preparation.evict();
    }

    pub fn retained(&self) -> usize {
        self.account.retained()
    }
}
