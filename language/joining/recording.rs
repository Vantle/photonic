use super::node::Node;
use super::playback::Playback;
use super::slot::Slot;
use super::trace::{self, Trace};
use crate::index::Index;
use std::sync::Arc;
use std::task::Poll;

pub(super) struct Recording {
    pub trace: Arc<Trace>,
    publication: Option<Arc<Trace>>,
    pub playback: Playback,
    writable: bool,
    threshold: usize,
}

impl Recording {
    pub fn new(trace: Trace) -> Self {
        Self {
            trace: Arc::new(trace),
            publication: None,
            playback: Playback::default(),
            writable: true,
            threshold: 256,
        }
    }

    pub fn shared(trace: Arc<Trace>) -> Self {
        Self {
            trace,
            publication: None,
            playback: Playback::default(),
            writable: false,
            threshold: 256,
        }
    }

    pub fn writable(&self) -> bool {
        self.writable
    }

    pub fn append(&mut self, result: &Poll<Option<Vec<Slot>>>) -> bool {
        if self.playback.progress == trace::LENGTH {
            return false;
        }
        let allowance = trace::CAPACITY - self.retained();
        let mut recorded = Arc::get_mut(&mut self.trace)
            .unwrap()
            .append(result, 0.., allowance);
        if !recorded && self.publication.take().is_some() {
            let allowance = trace::CAPACITY - self.trace.retained;
            recorded = Arc::get_mut(&mut self.trace)
                .unwrap()
                .append(result, 0.., allowance);
        }
        if recorded {
            self.playback.cursor = self.trace.record.len();
            self.playback.progress += 1;
        }
        recorded
    }

    pub fn retained(&self) -> usize {
        self.trace.retained + self.publication.as_ref().map_or(0, |trace| trace.retained)
    }

    pub fn publish(&mut self, node: &Node, index: &Index) {
        if self.trace.complete {
            self.publication = None;
            node.publish(index, &self.trace);
            return;
        }
        if self.trace.length < self.threshold {
            return;
        }
        self.threshold *= 2;
        self.publication = None;
        if let Some(trace) = self.trace.duplicate(trace::CAPACITY - self.trace.retained) {
            let trace = Arc::new(trace);
            node.publish(index, &trace);
            self.publication = Some(trace);
        }
    }

    pub fn share(&self, node: &Node, index: &Index) {
        if self.trace.complete || !self.writable {
            node.publish(index, &self.trace);
        } else if let Some(trace) = &self.publication {
            node.publish(index, trace);
        }
    }

    #[cfg(test)]
    pub fn size(&self) -> usize {
        self.trace.size() + self.publication.as_ref().map_or(0, |trace| trace.size())
    }
}
