use super::Flow;
use super::composition::Composition;
use super::key::Key;
use super::template::Template;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;

pub(crate) struct Store {
    entry: HashMap<Key, Composition>,
    template: HashMap<Key, Option<Template>>,
    capacity: usize,
    retained: usize,
    disabled: bool,
}

impl Store {
    pub fn new(capacity: usize) -> Self {
        Self {
            entry: HashMap::new(),
            template: HashMap::new(),
            capacity,
            retained: 0,
            disabled: false,
        }
    }

    pub fn compose(&mut self, parent: &Arc<Flow>, event: &Arc<Flow>) -> Flow {
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
        let template = Key::new(event);
        if !self.template.contains_key(&template) {
            self.template.retain(|key, value| {
                if key.alive() {
                    return true;
                }
                self.retained -= value.as_ref().map_or(1, Template::retained);
                false
            });
        }
        let template = match self.template.entry(template) {
            Entry::Vacant(entry) => {
                self.retained += 1;
                entry.insert(None)
            }
            Entry::Occupied(entry) => {
                let value = entry.into_mut();
                if value.is_none() {
                    let template = Template::new(event);
                    self.retained += template.retained() - 1;
                    *value = Some(template);
                }
                value
            }
        };
        let entry = self.entry.entry(key).or_insert_with(|| {
            self.retained += 1;
            Composition::default()
        });
        self.retained -= entry.retained();
        let result = match template {
            Some(template) => template.compose(parent, entry),
            None => entry.compose(parent, event),
        };
        self.retained += entry.retained();
        if self.retained > self.capacity {
            self.entry.clear();
            self.template.clear();
            self.retained = 0;
        }
        result
    }

    pub fn retained(&self) -> usize {
        self.retained
    }

    pub fn evict(&mut self) {
        self.entry.clear();
        self.template.clear();
        self.retained = 0;
        self.disabled = true;
    }
}
