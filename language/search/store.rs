use super::key::Key;
use super::transcript::Transcript;
use crate::budget::{Account, Reservation};
use crate::hashing::Builder;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub(super) struct Entry {
    pub transcript: Transcript,
    _reservation: Reservation,
}

pub(crate) struct Store {
    entry: Mutex<HashMap<Key, Arc<Entry>, Builder>>,
    capacity: usize,
    account: Account,
}

impl Store {
    pub fn new(capacity: usize) -> Self {
        Self {
            entry: Mutex::new(HashMap::default()),
            capacity,
            account: Account::new(capacity),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub(super) fn find(&self, key: &Key) -> Option<Arc<Entry>> {
        self.entry.lock().unwrap().get(key).cloned()
    }

    pub(super) fn insert(&self, key: Key, transcript: Transcript) {
        let mut entry = self.entry.lock().unwrap();
        if entry.contains_key(&key) {
            return;
        }
        entry.retain(|key, _| key.alive());
        let size = key.retained() + transcript.retained + 1;
        let Some(reservation) = self.account.reserve(size) else {
            return;
        };
        entry.insert(
            key,
            Arc::new(Entry {
                transcript,
                _reservation: reservation,
            }),
        );
    }

    pub fn retained(&self) -> usize {
        self.account.retained()
    }

    pub fn evict(&self) {
        self.entry.lock().unwrap().clear();
    }
}
