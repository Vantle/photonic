use super::capture::Capture;
use super::trace::Trace;
use super::{Laser, map, scan};
use crate::executor::Executor;
use crate::profile;
use std::ops::Range;
use std::sync::Arc;

impl Laser {
    pub(super) fn discover(
        &mut self,
        executor: Option<&Executor>,
        fresh: Vec<usize>,
    ) -> Vec<(usize, Range<usize>)> {
        let _scope = profile::Scope::new(profile::Phase::Discovery);
        let scanned = map(executor, fresh, |index| {
            let found = scan::scan(&self.catalog, &self.state[index]).collect::<Vec<_>>();
            (index, found)
        });
        scanned
            .into_iter()
            .map(|(index, found)| {
                self.seed(index, &found);
                (index, 0..self.origin[index])
            })
            .collect()
    }

    fn seed(&mut self, index: usize, found: &[scan::Match]) {
        let state = self.state[index].clone();
        for found in found {
            let capture = Capture::new(found.owner, &state, index, &mut self.pool)
                .map(|capture| self.capture.share(Arc::new(capture)));
            if let Some(trace) = Trace::initial(found, &state, capture, &mut self.pool) {
                self.trace[index].insert(trace);
            }
        }
        self.origin[index] = self.trace[index].len();
    }
}
