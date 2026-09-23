mod key;
mod node;
mod selection;

pub(crate) use node::Node;

use crate::budget::Account;
use crate::hashing::Builder;
use crate::index::Index;
use crate::term::Term;
use key::Key;
use selection::Selection;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub(crate) struct Request<'a, Pattern> {
    pub pattern: Pattern,
    pub index: &'a Index,
    pub frame: usize,
}

pub(crate) struct Change {
    pub affected: bool,
    pub insertion: Arc<Selection>,
}

pub(crate) struct Domain {
    pub site: Vec<usize>,
    pub node: Option<Arc<Node>>,
}

pub(crate) struct Store {
    account: Account,
    node: Mutex<HashMap<Arc<Key>, Arc<Node>, Builder>>,
}

impl Store {
    pub fn new(account: Account) -> Self {
        Self {
            account,
            node: Mutex::new(HashMap::default()),
        }
    }

    pub fn select(&self, request: Request<'_, impl IntoIterator<Item = Term>>) -> Domain {
        let key = Key::new(request.pattern, request.frame);
        let shared = self.node.lock().unwrap().get(&key).cloned();
        if let Some(node) = shared {
            return Domain {
                site: node.select(request.index).site.clone(),
                node: Some(node),
            };
        }
        let site = key.select(request.index);
        if site.len() <= 16 || site.len() >= 4096 {
            return Domain { site, node: None };
        }
        let mut node = self.node.lock().unwrap();
        if let Some(node) = node.get(&key) {
            return Domain {
                site,
                node: Some(node.clone()),
            };
        }
        let reservation = self.account.reserve(key.retained()).or_else(|| {
            node.retain(|_, node| Arc::strong_count(node) > 1);
            self.account.reserve(key.retained())
        });
        let Some(reservation) = reservation else {
            return Domain { site, node: None };
        };
        let key = Arc::new(key);
        let entry = Arc::new(Node::new(key.clone(), self.account.clone(), reservation));
        let selection = Selection::new(site, &self.account);
        entry.publish(request.index, &selection);
        node.insert(key, entry.clone());
        Domain {
            site: selection.site.clone(),
            node: Some(entry),
        }
    }

    pub fn evict(&self) {
        self.node.lock().unwrap().clear();
    }
}

#[cfg(test)]
#[path = "test/candidate.rs"]
mod test;
