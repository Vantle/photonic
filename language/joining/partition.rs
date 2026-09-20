use super::cursor::Cursor;
use super::playback::Playback;
use super::prefix::Prefix;
use super::space::Space;
use super::trace::Trace;
use crate::factor::Budget;
use crate::index::Index;
use crate::slot::Slot;
use std::collections::HashMap;
use std::sync::Arc;
use std::task::Poll;

struct Active {
    site: usize,
    trace: Box<Trace>,
}

pub(super) struct Partition {
    source: Cursor,
    budget: Arc<Budget>,
    record: HashMap<usize, Box<Trace>>,
    active: Option<Active>,
    playback: Playback,
    restoration: Option<usize>,
    cached: usize,
    enabled: bool,
}

impl Partition {
    pub fn new(width: usize, budget: Arc<Budget>) -> Self {
        Self {
            source: Cursor::new(width),
            budget,
            record: HashMap::new(),
            active: None,
            playback: Playback::default(),
            restoration: None,
            cached: 0,
            enabled: true,
        }
    }

    fn finish(&mut self) {
        if let Some(active) = self.active.take() {
            if active.trace.complete {
                self.record.insert(active.site, active.trace);
            } else {
                self.cached -= active.trace.retained;
            }
        }
        self.playback = Playback::default();
    }

    pub fn update(&mut self, index: &Index) {
        self.reset(index);
        for site in &index.removal {
            if let Some(trace) = self.record.remove(site) {
                self.cached -= trace.retained;
            }
        }
    }

    fn prepare(&mut self, space: &Space, order: &[usize]) {
        if !self.enabled || self.active.is_some() {
            return;
        }
        let Some(position) = self.source.boundary() else {
            return;
        };
        let Some(member) = space.domain[order[0]].get(position) else {
            return;
        };
        let trace = self.record.remove(&member.site).or_else(|| {
            if self.cached > 4094 {
                return None;
            }
            let trace = Trace::new(self.budget.clone(), 2)?;
            self.cached += trace.retained;
            Some(Box::new(trace))
        });
        self.active = trace.map(|trace| Active {
            site: member.site,
            trace,
        });
    }

    fn advance(
        &mut self,
        space: &mut Space,
        order: &[usize],
        index: &Index,
    ) -> Poll<Option<Vec<Slot>>> {
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.trace.complete)
        {
            let position = self.source.boundary().unwrap();
            self.source.seek(position + 1);
            self.finish();
        }
        if let Some(progress) = self.restoration.take() {
            for _ in 0..progress {
                let _ = self.source.step(space, order, index);
            }
        }
        self.prepare(space, order);
        if let Some(active) = &self.active
            && active.trace.complete
        {
            return self.playback.step(&active.trace, order).unwrap();
        }
        let result = self.source.step(space, order, index);
        let Some(active) = &mut self.active else {
            return result;
        };
        let previous = active.trace.retained;
        if self.playback.progress == 65536 || !active.trace.append(&result, 4096 - self.cached) {
            self.finish();
            return result;
        }
        self.cached += active.trace.retained - previous;
        self.playback.progress += 1;
        if self.source.boundary().is_some() {
            active.trace.complete = true;
            self.finish();
        }
        result
    }
}

impl super::prefix::Prefix for Partition {
    fn reset(&mut self, _: &Index) {
        self.finish();
        self.source.reset();
        self.restoration = None;
    }

    #[inline]
    fn step(
        &mut self,
        space: &mut Space,
        order: &[usize],
        index: &Index,
    ) -> Poll<Option<Vec<Slot>>> {
        if let Some(active) = &self.active
            && active.trace.complete
            && let Some(result) = self.playback.step(&active.trace, order)
        {
            return result;
        }
        self.advance(space, order, index)
    }

    fn evict(&mut self) {
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.trace.complete)
        {
            self.restoration = Some(self.playback.progress);
        }
        self.record.clear();
        self.active = None;
        self.playback = Playback::default();
        self.cached = 0;
        self.enabled = false;
    }

    fn cached(&self) -> usize {
        self.cached
    }

    fn retained(&self) -> usize {
        self.source.retained() + self.cached() + 1
    }

    #[cfg(test)]
    fn size(&self) -> usize {
        self.source.size()
            + self
                .record
                .values()
                .map(|trace| trace.size())
                .sum::<usize>()
            + self.active.as_ref().map_or(0, |active| active.trace.size())
            + 1
    }
}
