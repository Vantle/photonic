use super::Constraint;
use crate::hashing::Builder;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Copy)]
pub(super) struct Identity {
    generation: usize,
    position: usize,
}

#[derive(Eq, Hash, PartialEq)]
struct Key {
    parent: Option<usize>,
    constraint: Constraint,
}

#[derive(Clone, Copy)]
pub(super) struct Decision {
    pub accepted: bool,
    pub identity: Option<Identity>,
}

#[derive(Default)]
struct Cache {
    generation: usize,
    entry: HashMap<Key, Decision, Builder>,
}

pub(crate) struct Store {
    retained: AtomicUsize,
    capacity: usize,
    cache: Mutex<Cache>,
}

impl Store {
    pub fn eligible(width: usize) -> bool {
        width >= 512
    }

    pub fn new(capacity: usize) -> Self {
        Self {
            retained: AtomicUsize::new(0),
            capacity,
            cache: Mutex::new(Cache::default()),
        }
    }

    pub(super) fn select(
        &self,
        parent: Option<Identity>,
        constraint: Constraint,
        evaluate: impl FnOnce() -> bool,
    ) -> Decision {
        let mut cache = self.cache.lock().unwrap();
        if cache.generation == usize::MAX
            || parent.is_some_and(|parent| parent.generation != cache.generation)
        {
            drop(cache);
            return Decision {
                accepted: evaluate(),
                identity: None,
            };
        }
        let key = Key {
            parent: parent.map(|parent| parent.position),
            constraint,
        };
        if let Some(decision) = cache.entry.get(&key) {
            return *decision;
        }
        let accepted = evaluate();
        if cache.entry.len() >= self.capacity / 6 {
            return Decision {
                accepted,
                identity: None,
            };
        }
        let identity = accepted.then_some(Identity {
            generation: cache.generation,
            position: cache.entry.len(),
        });
        let decision = Decision { accepted, identity };
        cache.entry.insert(key, decision);
        self.retained
            .store(cache.entry.len() * 6, Ordering::Relaxed);
        decision
    }

    pub fn evict(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.entry.clear();
        self.retained.store(0, Ordering::Relaxed);
        cache.generation = cache.generation.saturating_add(1);
    }

    pub fn retained(&self) -> usize {
        self.retained.load(Ordering::Relaxed)
    }
}
