use super::cursor::Cursor;
use super::node::Node;
use super::playback::Playback;
use super::space::Space;
use super::trace::Trace;
use crate::index::Index;
use crate::slot::Slot;
use std::sync::Arc;
use std::task::Poll;

enum Mode {
    Fresh,
    Visited,
    Repeated,
    Streaming,
    Recording(Box<Cache>),
}

struct Cache {
    trace: Arc<Trace>,
    playback: Playback,
}

pub(super) struct Stream {
    source: Cursor,
    budget: Arc<crate::factor::Budget>,
    node: Option<Arc<Node>>,
    mode: Mode,
    restoration: Option<usize>,
}

impl Stream {
    pub fn new(width: usize, budget: Arc<crate::factor::Budget>, node: Option<Arc<Node>>) -> Self {
        Self {
            source: Cursor::new(width),
            budget,
            node,
            mode: Mode::Fresh,
            restoration: None,
        }
    }

    fn shared(&self) -> Option<&Node> {
        self.node
            .as_ref()
            .filter(|node| Arc::strong_count(node) > 2)
            .map(Arc::as_ref)
    }

    fn advance(
        &mut self,
        space: &mut Space,
        order: &[usize],
        index: &Index,
    ) -> Poll<Option<Vec<Slot>>> {
        if let Some(progress) = self.restoration.take() {
            for _ in 0..progress {
                let _ = self.source.step(space, order, index);
            }
        }
        if matches!(self.mode, Mode::Fresh) {
            self.mode = Mode::Visited;
        }
        if matches!(self.mode, Mode::Repeated) {
            self.mode = if let Some(trace) = Trace::new(self.budget.clone(), 1) {
                Mode::Recording(Box::new(Cache {
                    trace: Arc::new(trace),
                    playback: Playback::default(),
                }))
            } else {
                Mode::Streaming
            };
            if matches!(self.mode, Mode::Recording(_)) {
                return self.record(space, order, index);
            }
        }
        self.source.step(space, order, index)
    }

    fn record(
        &mut self,
        space: &mut Space,
        order: &[usize],
        index: &Index,
    ) -> Poll<Option<Vec<Slot>>> {
        let result = self.source.step(space, order, index);
        let Mode::Recording(cache) = &mut self.mode else {
            unreachable!();
        };
        if cache.playback.progress == 65536
            || !Arc::get_mut(&mut cache.trace)
                .unwrap()
                .append(&result, 4096)
        {
            self.mode = Mode::Streaming;
            return result;
        }
        cache.playback.cursor = cache.trace.record.len();
        cache.playback.progress += 1;
        if cache.trace.complete
            && let Some(node) = &self.node
            && Arc::strong_count(node) > 2
        {
            node.publish(index, &cache.trace);
        }
        result
    }

    #[cfg(test)]
    pub fn shares(&self, other: &Self) -> bool {
        match (&self.mode, &other.mode) {
            (Mode::Recording(left), Mode::Recording(right)) => {
                Arc::ptr_eq(&left.trace, &right.trace)
            }
            _ => false,
        }
    }
}

impl super::prefix::Prefix for Stream {
    fn reset(&mut self, index: &Index) {
        if let Mode::Recording(cache) = &mut self.mode {
            cache.playback = Playback::default();
            if cache.trace.complete
                && let Some(node) = &self.node
                && Arc::strong_count(node) > 2
            {
                node.publish(index, &cache.trace);
            }
            return;
        }
        self.restoration = None;
        self.source.reset();
        if let Some(trace) = self.shared().and_then(|node| node.find(index)) {
            self.mode = Mode::Recording(Box::new(Cache {
                trace,
                playback: Playback::default(),
            }));
        } else if matches!(self.mode, Mode::Visited) {
            self.mode = Mode::Repeated;
        }
    }

    #[inline]
    fn step(
        &mut self,
        space: &mut Space,
        order: &[usize],
        index: &Index,
    ) -> Poll<Option<Vec<Slot>>> {
        let cache = match &mut self.mode {
            Mode::Recording(cache) => cache,
            Mode::Visited => return self.source.step(space, order, index),
            Mode::Streaming if self.restoration.is_none() => {
                return self.source.step(space, order, index);
            }
            _ => return self.advance(space, order, index),
        };
        if let Some(result) = cache.playback.step(&cache.trace, order) {
            return result;
        }
        if cache.trace.complete {
            return Poll::Ready(None);
        }
        self.record(space, order, index)
    }

    fn evict(&mut self) {
        self.node = None;
        let Mode::Recording(cache) = &self.mode else {
            return;
        };
        self.restoration = Some(cache.playback.progress);
        self.source.reset();
        self.mode = Mode::Streaming;
    }

    #[cfg(test)]
    fn size(&self) -> usize {
        self.source.size()
            + 1
            + match &self.mode {
                Mode::Recording(cache) => cache.trace.size(),
                _ => 0,
            }
    }

    fn cached(&self) -> usize {
        match &self.mode {
            Mode::Recording(cache) => cache.trace.retained,
            _ => 0,
        }
    }

    fn retained(&self) -> usize {
        self.source.retained() + self.cached() + 1
    }
}
