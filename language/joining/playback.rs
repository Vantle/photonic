use super::trace::{Record, Trace};
use crate::slot::Slot;
use std::task::Poll;

#[derive(Default)]
pub(super) struct Playback {
    pub cursor: usize,
    waiting: usize,
    pub progress: usize,
}

impl Playback {
    #[inline]
    pub fn step(&mut self, trace: &Trace, order: &[usize]) -> Option<Poll<Option<Vec<Slot>>>> {
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
                Poll::Ready(Some(
                    binding
                        .iter()
                        .enumerate()
                        .map(|(position, selection)| Slot {
                            world: selection.site,
                            position: order[position],
                            token: selection.token.clone(),
                        })
                        .collect(),
                ))
            }
        })
    }
}
