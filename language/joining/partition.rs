use super::cursor::Cursor;
use super::dependency::Dependency;
use super::playback::Playback;
use super::prefix::Prefix;
use super::retention::Retention;
use super::slot::Slot;
use super::space::Space;
use super::trace::{self, Trace};
use crate::factor::Budget;
use crate::index::Index;
use std::sync::Arc;
use std::task::Poll;

struct Active {
    dependency: Arc<Dependency>,
    trace: Box<Trace>,
}

pub(super) struct Partition {
    source: Cursor,
    depth: usize,
    budget: Arc<Budget>,
    record: Retention,
    active: Option<Active>,
    playback: Playback,
    restoration: Option<usize>,
    cached: usize,
    enabled: bool,
}

impl Partition {
    pub fn new(width: usize, depth: usize, budget: Arc<Budget>) -> Self {
        Self {
            source: Cursor::new(width),
            depth,
            budget,
            record: Retention::default(),
            active: None,
            playback: Playback::default(),
            restoration: None,
            cached: 0,
            enabled: true,
        }
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn granularity(&self) -> Option<usize> {
        self.record.granularity()
    }

    fn finish(&mut self) {
        if let Some(active) = self.active.take() {
            if active.trace.complete {
                self.record.insert(active.dependency, active.trace);
            } else {
                self.cached -= active.trace.retained;
            }
        }
        self.playback = Playback::default();
    }

    pub fn update(&mut self, index: &Index) {
        self.reset(index);
        for site in &index.removal {
            self.cached -= self.record.remove(*site);
        }
    }

    fn prepare(&mut self, space: &Space, order: &[usize], index: &Index) {
        if !self.enabled || self.active.is_some() {
            return;
        }
        let Some(position) = self.source.boundary(self.depth) else {
            return;
        };
        let Some(member) = space.domain[order[self.depth]].get(position) else {
            return;
        };
        let dependency = space.dependency(member.site, self.source.binding(), order, index);
        let record = self.record.take(&dependency).or_else(|| {
            let retained = dependency.retained();
            if retained > trace::CAPACITY - self.cached {
                return None;
            }
            let trace = Trace::new(self.budget.clone(), retained)?;
            self.cached += trace.retained;
            Some((Arc::new(dependency), Box::new(trace)))
        });
        self.active = record.map(|(dependency, trace)| Active { dependency, trace });
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
            let position = self.source.boundary(self.depth).unwrap();
            self.source.seek(self.depth, position + 1);
            self.finish();
        }
        if let Some(progress) = self.restoration.take() {
            for _ in 0..progress {
                let _ = self.source.step(space, order, index);
            }
        }
        self.prepare(space, order, index);
        if let Some(active) = &self.active
            && active.trace.complete
        {
            return self
                .playback
                .step(&active.trace, order, self.source.binding())
                .unwrap();
        }
        let result = self.source.step(space, order, index);
        let Some(active) = &mut self.active else {
            return result;
        };
        let previous = active.trace.retained;
        if self.playback.progress == trace::LENGTH
            || !active
                .trace
                .append(&result, self.depth.., trace::CAPACITY - self.cached)
        {
            self.finish();
            return result;
        }
        self.cached += active.trace.retained - previous;
        self.playback.progress += 1;
        if self.source.boundary(self.depth).is_some() {
            active.trace.complete = true;
            self.finish();
        }
        result
    }
}

impl super::prefix::Prefix for Partition {
    fn waiting(&self) -> usize {
        self.active
            .as_ref()
            .filter(|active| active.trace.complete)
            .map_or(0, |active| self.playback.waiting(&active.trace))
    }

    fn skip(&mut self, maximum: usize) -> usize {
        self.active
            .as_ref()
            .filter(|active| active.trace.complete)
            .map_or(0, |active| self.playback.skip(&active.trace, maximum))
    }

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
            && let Some(result) = self
                .playback
                .step(&active.trace, order, self.source.binding())
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
            + self.record.size(
                self.active
                    .as_ref()
                    .filter(|active| active.trace.complete)
                    .map(|active| active.dependency.as_ref()),
            )
            + self.active.as_ref().map_or(0, |active| active.trace.size())
            + 1
    }
}
