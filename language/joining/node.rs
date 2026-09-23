use super::trace::Trace;
use crate::budget::Reservation;
use crate::index::Index;
use crate::revision::Revision;
use std::sync::{Arc, Mutex, Weak};

struct Version {
    revision: Revision,
    trace: Weak<Trace>,
}

pub(super) struct Node {
    version: Mutex<Option<Version>>,
    _reservation: Reservation,
}

impl Node {
    pub fn new(reservation: Reservation) -> Self {
        Self {
            version: Mutex::new(None),
            _reservation: reservation,
        }
    }

    pub fn find(&self, index: &Index) -> Option<Arc<Trace>> {
        let version = self.version.lock().unwrap();
        let version = version.as_ref()?;
        if version.revision != *index.revision() {
            return None;
        }
        version.trace.upgrade()
    }

    pub fn publish(&self, index: &Index, trace: &Arc<Trace>) {
        let mut version = self.version.lock().unwrap();
        if let Some(previous) = version
            .as_ref()
            .filter(|version| version.revision == *index.revision())
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
