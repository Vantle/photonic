use super::trace::{Record, Selection, Trace};
use crate::slot::Slot;
use std::task::Poll;

#[derive(Default)]
pub(super) struct Playback {
    pub cursor: usize,
    waiting: usize,
    pub progress: usize,
}

impl Playback {
    pub fn seek(&mut self, trace: &Trace, progress: usize) {
        *self = Self::default();
        self.progress = progress;
        let mut remaining = progress;
        while remaining != 0 {
            let length = match &trace.record[self.cursor] {
                Record::Waiting(count) => *count,
                Record::Binding(_) => 1,
            };
            self.cursor += 1;
            if length > remaining {
                self.waiting = length - remaining;
                return;
            }
            remaining -= length;
        }
    }

    #[inline]
    pub fn step(
        &mut self,
        trace: &Trace,
        order: &[usize],
        prefix: &[Slot],
    ) -> Option<Poll<Option<Vec<Slot>>>> {
        if self.waiting != 0 {
            self.waiting -= 1;
            self.progress += 1;
            return Some(Poll::Pending);
        }
        let record = trace.record.get(self.cursor)?;
        self.progress += 1;
        Some(match record {
            Record::Waiting(count) => {
                self.waiting = count - 1;
                self.cursor += 1;
                Poll::Pending
            }
            Record::Binding(binding) => {
                self.cursor += 1;
                Poll::Ready(Some(Self::binding(binding, order, prefix)))
            }
        })
    }

    #[inline(never)]
    fn binding(binding: &[Selection], order: &[usize], prefix: &[Slot]) -> Vec<Slot> {
        prefix
            .iter()
            .cloned()
            .chain(
                binding
                    .iter()
                    .enumerate()
                    .map(|(position, selection)| Slot {
                        world: selection.site,
                        position: order[prefix.len() + position],
                        token: selection.token.clone(),
                    }),
            )
            .collect()
    }
}
