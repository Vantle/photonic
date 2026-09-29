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
        let shard = &self.shard[shard::slot(hashing::value(&key))];
        if let Some(found) = shard.lock().expect("an unpoisoned memo").get(&key) {
            return found.clone();
        }
        let made = make(&key);
        shard
            .lock()
            .expect("an unpoisoned memo")
            .entry(key)
            .or_insert(made)
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
