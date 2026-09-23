use super::dependency::Dependency;
use super::retention::Retention;
use super::slot::Slot;
use super::trace::{self, Trace};
use crate::factor::Budget;
use crate::index::Index;
use std::sync::Arc;
use std::task::Poll;

pub(super) struct Active {
    pub dependency: Arc<Dependency>,
    pub trace: Box<Trace>,
}

pub(super) struct Layer {
    pub depth: usize,
    pub cached: usize,
    record: Retention,
    active: Option<Active>,
}

impl Layer {
    pub fn new(depth: usize) -> Self {
        Self {
            depth,
            cached: 0,
            record: Retention::default(),
            active: None,
        }
    }

    pub fn prepare(&mut self, dependency: Dependency, budget: &Arc<Budget>, allowance: usize) {
        if self.active.is_some() {
            return;
        }
        let record = self.record.take(&dependency).or_else(|| {
            let retained = dependency.retained();
            if retained > allowance {
                return None;
            }
            let trace = Trace::new(budget.clone(), retained)?;
            self.cached += trace.retained;
            Some((Arc::new(dependency), Box::new(trace)))
        });
        self.active = record.map(|(dependency, trace)| Active { dependency, trace });
    }

    pub fn take(&mut self) -> Option<Active> {
        if self
            .active
            .as_ref()
            .is_some_and(|active| active.trace.complete)
        {
            self.active.take()
        } else {
            None
        }
    }

    pub fn restore(&mut self, active: Active) {
        self.record.insert(active.dependency, active.trace);
    }

    pub fn append(&mut self, result: &Poll<Option<Vec<Slot>>>, allowance: usize) {
        let Some(active) = self.active.as_mut().filter(|active| !active.trace.complete) else {
            return;
        };
        let previous = active.trace.retained;
        if active.trace.length >= trace::LENGTH
            || !active.trace.append(result, self.depth.., allowance)
        {
            self.finish();
            return;
        }
        self.cached += active.trace.retained - previous;
    }

    pub fn seal(&mut self) {
        if let Some(active) = self.active.as_mut().filter(|active| !active.trace.complete) {
            active.trace.complete = true;
            self.finish();
        }
    }

    pub fn extend(&mut self, trace: &Trace, prefix: &[Slot], allowance: usize) {
        let Some(active) = self.active.as_mut().filter(|active| !active.trace.complete) else {
            return;
        };
        let previous = active.trace.retained;
        if active.trace.length >= trace::LENGTH || !active.trace.extend(trace, prefix, allowance) {
            self.finish();
            return;
        }
        self.cached += active.trace.retained - previous;
    }

    pub fn finish(&mut self) {
        let Some(active) = self.active.take() else {
            return;
        };
        if active.trace.complete {
            self.record.insert(active.dependency, active.trace);
        } else {
            self.cached -= active.trace.retained;
        }
    }

    pub fn update(&mut self, index: &Index, invalid: bool) {
        if invalid {
            self.clear();
            return;
        }
        for site in &index.removal {
            self.cached -= self.record.remove(*site);
        }
    }

    pub fn clear(&mut self) {
        self.record.clear();
        self.active = None;
        self.cached = 0;
    }

    #[cfg(test)]
    pub fn size(&self, active: Option<&Active>) -> usize {
        let inherited = active;
        let active = active.or_else(|| self.active.as_ref().filter(|active| active.trace.complete));
        self.record
            .size(active.map(|active| active.dependency.as_ref()))
            + self.active.as_ref().map_or(0, |active| active.trace.size())
            + inherited.map_or(0, |active| active.trace.size())
    }
}
