use super::key::Key;
use super::trace::Trace;
use crate::factor::Budget;
use crate::index::Index;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};

struct Version {
    revision: Arc<()>,
    trace: Weak<Trace>,
}

pub(super) struct Node {
    key: Arc<Key>,
    budget: Arc<Budget>,
    accounting: Arc<AtomicUsize>,
    version: Mutex<Option<Version>>,
}

impl Node {
    pub fn new(key: Arc<Key>, budget: Arc<Budget>, accounting: Arc<AtomicUsize>) -> Self {
        accounting.fetch_add(key.retained(), Ordering::Relaxed);
        Self {
            key,
            budget,
            accounting,
            version: Mutex::new(None),
        }
    }

    pub fn find(&self, index: &Index) -> Option<Arc<Trace>> {
        let version = self.version.lock().unwrap();
        let version = version.as_ref()?;
        if !Arc::ptr_eq(&version.revision, index.revision()) {
            return None;
        }
        version.trace.upgrade()
    }

    pub fn publish(&self, index: &Index, trace: &Arc<Trace>) {
        let mut version = self.version.lock().unwrap();
        if let Some(previous) = version
            .as_ref()
            .filter(|version| Arc::ptr_eq(&version.revision, index.revision()))
            .and_then(|version| version.trace.upgrade())
            && (previous.complete || previous.length > trace.length)
        {
            return;
        }
        *version = Some(Version {
            revision: index.revision().clone(),
            trace: Arc::downgrade(trace),
        });
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        let retained = self.key.retained();
        self.budget.release(retained);
        self.accounting.fetch_sub(retained, Ordering::Relaxed);
    }
}
