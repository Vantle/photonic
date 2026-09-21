use crate::basis::Set;
use std::collections::HashMap;
use std::hash::Hash;

pub(super) struct Union<Value> {
    entry: HashMap<Set<Value>, Set<Value>>,
    retained: usize,
}

impl<Value> Default for Union<Value> {
    fn default() -> Self {
        Self {
            entry: HashMap::new(),
            retained: 0,
        }
    }
}

impl<Value: Clone + Eq + Hash> Union<Value> {
    pub fn select(
        &mut self,
        source: &Set<Value>,
        compute: impl FnOnce() -> Set<Value>,
    ) -> Set<Value> {
        if let Some(value) = self.entry.get(source) {
            return value.clone();
        }
        let value = compute();
        self.retained += 2 + source.len() + value.len();
        self.entry.insert(source.clone(), value.clone());
        value
    }

    pub fn retained(&self) -> usize {
        self.retained
    }
}
