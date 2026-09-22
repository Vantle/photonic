use super::key::Key;
use super::node::Node;
use crate::factor::Budget;
use crate::hashing::Builder;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub(crate) struct Store {
    pub domain: crate::candidate::Store,
    pub preparation: Arc<crate::preparation::Store>,
    budget: Arc<Budget>,
    accounting: Arc<AtomicUsize>,
    node: Mutex<HashMap<Arc<Key>, Arc<Node>, Builder>>,
}

impl Store {
    pub fn new(capacity: usize) -> Self {
        let budget = Arc::new(Budget::new(capacity));
        let accounting = Arc::new(AtomicUsize::new(0));
        Self {
            domain: crate::candidate::Store::new(budget.clone(), accounting.clone()),
            preparation: Arc::new(crate::preparation::Store::new(
                budget.clone(),
                accounting.clone(),
            )),
            budget,
            accounting,
            node: Mutex::new(HashMap::default()),
        }
    }

    pub fn budget(&self) -> &Arc<Budget> {
        &self.budget
    }

    pub(super) fn subscribe(&self, key: Key) -> Option<Arc<Node>> {
        let mut node = self.node.lock().unwrap();
        if let Some(node) = node.get(&key) {
            return Some(node.clone());
        }
        if !self.budget.reserve(key.retained()) {
            node.retain(|_, node| Arc::strong_count(node) > 1);
            if !self.budget.reserve(key.retained()) {
                return None;
            }
        }
        let key = Arc::new(key);
        let entry = Arc::new(Node::new(
            key.clone(),
            self.budget.clone(),
            self.accounting.clone(),
        ));
        node.insert(key, entry.clone());
        Some(entry)
    }

    pub fn evict(&self) {
        self.node.lock().unwrap().clear();
        self.domain.evict();
        self.preparation.evict();
    }

    pub fn retained(&self) -> usize {
        self.accounting.load(Ordering::Relaxed)
    }
}
