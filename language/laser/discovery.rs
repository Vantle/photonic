use super::capture::Capture;
use super::trace::Trace;
use super::{Laser, map, scan};
use crate::executor::Executor;
use crate::profile;
use std::collections::VecDeque;
use std::ops::Range;
use std::sync::Arc;

// A group of scans holds every match its configurations hold until they become traces, so
// configurations are scanned a group of about this many matches at a time, as many as the
// configurations scanned so far held on average, and scanning stops between groups once the
// retained records reach the record limit.
const SCANNING: usize = 1 << 16;

impl Laser {
    // Scans at most the allowance of the fresh configurations, in order, and gives each one's own
    // traces with the configurations a budget left for the next round.
    pub(super) fn discover(
        &mut self,
        executor: Option<&Executor>,
        fresh: Vec<usize>,
        allowance: usize,
    ) -> (Vec<(usize, Range<usize>)>, Vec<usize>) {
        let _scope = profile::Scope::new(profile::Phase::Discovery);
        let mut scanned = self.state.len() - fresh.len();
        let mut queue = VecDeque::from(fresh);
        let mut novel = Vec::new();
        while !queue.is_empty() && novel.len() < allowance && self.retained() < self.limit.record {
            let average = self.found.div_ceil(scanned.max(1)).max(1);
            let size = (SCANNING / average)
                .max(1)
                .min(allowance - novel.len())
                .min(queue.len());
            scanned += size;
            let group = queue.drain(..size).collect::<Vec<_>>();
            let result = map(executor, group, |index| {
                let found = scan::scan(&self.catalog, &self.state[index]).collect::<Vec<_>>();
                (index, found)
            });
            let mut unseeded = Vec::new();
            for (index, found) in result {
                if !unseeded.is_empty() || self.retained() >= self.limit.record {
                    unseeded.push(index);
                    continue;
                }
                self.seed(index, &found);
                novel.push((index, 0..self.origin[index]));
            }
            scanned -= unseeded.len();
            for index in unseeded.into_iter().rev() {
                queue.push_front(index);
            }
        }
        (novel, queue.into())
    }

    fn seed(&mut self, index: usize, found: &[scan::Match]) {
        let state = &self.state[index];
        for found in found {
            let capture = Capture::new(found.owner, state, index, &mut self.pool)
                .map(|capture| self.capture.share(Arc::new(capture)));
            if let Some(trace) = Trace::initial(found, state, capture, &mut self.pool) {
                self.trace[index].insert(trace);
            }
        }
        self.origin[index] = self.trace[index].len();
        self.traced += self.origin[index];
        self.found += self.origin[index];
    }
}
