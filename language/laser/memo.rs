use super::{map, shard};
use crate::executor::Executor;
use hashing::Builder;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Mutex;

// Values made from their keys and remembered, shared by workers. A value is made outside the lock,
// so workers make different values at once, and the first one stored is the one every worker gets.
pub(super) struct Memo<Key, Value> {
    shard: Vec<Mutex<HashMap<Key, Value, Builder>>>,
}

impl<Key, Value> Default for Memo<Key, Value> {
    fn default() -> Self {
        Self {
            shard: shard::empty(),
        }
    }
}

impl<Key: Eq + Hash, Value: Clone> Memo<Key, Value> {
    pub fn get(&self, key: Key, make: impl FnOnce(&Key) -> Value) -> Value {
        if let Some(found) = self.find(&key) {
            return found;
        }
        let made = make(&key);
        self.keep(key, made)
    }

    pub fn find(&self, key: &Key) -> Option<Value> {
        self.shard[shard::slot(hashing::value(key))]
            .lock()
            .expect("an unpoisoned memo")
            .get(key)
            .cloned()
    }

    // Remembers a value unless a worker remembered one first, and gives the one remembered.
    pub fn keep(&self, key: Key, value: Value) -> Value {
        self.shard[shard::slot(hashing::value(&key))]
            .lock()
            .expect("an unpoisoned memo")
            .entry(key)
            .or_insert(value)
            .clone()
    }
}

impl<Key: Send, Value: Send> Memo<Key, Value> {
    // A memo can hold millions of values, so its shards are dropped in parallel.
    pub fn forget(&mut self, executor: Option<&Executor>) {
        let shard = std::mem::take(self).shard;
        map(executor, shard, drop);
    }
}
