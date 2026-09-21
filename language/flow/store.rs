use super::Flow;
use super::composition::Composition;
use super::key::Key;
use std::collections::HashMap;
use std::sync::Arc;

pub(crate) struct Store {
    entry: HashMap<Key, Composition>,
    capacity: usize,
    retained: usize,
    disabled: bool,
}

impl Store {
    pub fn new(capacity: usize) -> Self {
        Self {
            entry: HashMap::new(),
            capacity,
            retained: 0,
            disabled: false,
        }
    }

    pub fn compose(&mut self, parent: &Arc<Flow>, event: &Flow) -> Flow {
        if self.disabled
            || event.resource.len() < 32
            || !event.resource.iter().any(|(_, source)| source.len() >= 8)
        {
            return parent.compose(event);
        }
        let key = Key::new(parent);
        if !self.entry.contains_key(&key) {
            self.entry.retain(|key, value| {
                if key.alive() {
                    return true;
                }
                self.retained -= value.retained();
                false
            });
        }
        let entry = self.entry.entry(key).or_insert_with(|| {
            self.retained += 1;
            Composition::default()
        });
        self.retained -= entry.retained();
        let result = entry.compose(parent, event);
        self.retained += entry.retained();
        if self.retained > self.capacity {
            self.entry.clear();
            self.retained = 0;
        }
        result
    }

    pub fn retained(&self) -> usize {
        self.retained
    }

    pub fn evict(&mut self) {
        self.entry.clear();
        self.retained = 0;
        self.disabled = true;
    }
}
