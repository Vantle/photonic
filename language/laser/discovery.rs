use super::capture::Capture;
use super::trace::Trace;
use super::{Laser, map, scan};
use crate::executor::Executor;
use crate::profile;
use std::collections::VecDeque;
use std::ops::Range;
use std::sync::Arc;

// Scanning keeps every match a configuration holds, so configurations are scanned a group at a time
// and scanning stops between them once the retained records reach the record limit.
const SCANNING: usize = 1 << 10;

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
        let mut queue = VecDeque::from(fresh);
        let mut novel = Vec::new();
        while !queue.is_empty() && novel.len() < allowance && self.retained() < self.limit.record {
            let size = SCANNING.min(allowance - novel.len()).min(queue.len());
            let group = queue.drain(..size).collect::<Vec<_>>();
            let scanned = map(executor, group, |index| {
                let found = scan::scan(&self.catalog, &self.state[index]).collect::<Vec<_>>();
                (index, found)
            });
            let mut unseeded = Vec::new();
            for (index, found) in scanned {
                if !unseeded.is_empty() || self.retained() >= self.limit.record {
                    unseeded.push(index);
                    continue;
                }
                self.seed(index, &found);
                novel.push((index, 0..self.origin[index]));
            }
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
    }
}
