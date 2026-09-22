use super::dependency::Dependency;
use super::trace::Trace;
use crate::hashing::Builder;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Default)]
pub(super) struct Retention {
    record: HashMap<Arc<Dependency>, Box<Trace>, Builder>,
    reverse: HashMap<usize, HashSet<Arc<Dependency>, Builder>, Builder>,
}

impl Retention {
    pub fn granularity(&self) -> Option<usize> {
        (!self.record.is_empty()).then(|| {
            self.record
                .values()
                .map(|trace| trace.length)
                .sum::<usize>()
                / self.record.len()
        })
    }

    pub fn take(&mut self, dependency: &Dependency) -> Option<(Arc<Dependency>, Box<Trace>)> {
        self.record.remove_entry(dependency)
    }

    pub fn insert(&mut self, dependency: Arc<Dependency>, trace: Box<Trace>) {
        for site in dependency.site() {
            self.reverse
                .entry(site)
                .or_default()
                .insert(dependency.clone());
        }
        self.record.insert(dependency, trace);
    }

    pub fn remove(&mut self, site: usize) -> usize {
        let Some(dependency) = self.reverse.remove(&site) else {
            return 0;
        };
        let mut retained = 0;
        for dependency in dependency {
            if let Some(trace) = self.record.remove(&dependency) {
                retained += trace.retained;
            }
            for site in dependency.site() {
                if let Some(entry) = self.reverse.get_mut(&site) {
                    entry.remove(&dependency);
                    if entry.is_empty() {
                        self.reverse.remove(&site);
                    }
                }
            }
        }
        retained
    }

    pub fn clear(&mut self) {
        self.record.clear();
        self.reverse.clear();
    }

    #[cfg(test)]
    pub fn size(&self, active: Option<&Dependency>) -> usize {
        for dependency in self.record.keys() {
            for site in dependency.site() {
                assert!(
                    self.reverse
                        .get(&site)
                        .is_some_and(|entry| entry.contains(dependency))
                );
            }
        }
        for (site, entry) in &self.reverse {
            assert!(!entry.is_empty());
            for dependency in entry {
                assert!(
                    self.record.contains_key(dependency) || active == Some(dependency.as_ref())
                );
                assert!(dependency.site().any(|value| value == *site));
            }
        }
        self.record.values().map(|trace| trace.size()).sum()
    }
}
