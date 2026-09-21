use crate::slot::Slot;
use std::task::Poll;

pub(super) enum Record {
    Waiting(usize),
    Binding(Vec<Slot>),
}

#[derive(Default)]
pub(super) struct Transcript {
    pub record: Vec<Record>,
    pub retained: usize,
}

impl Transcript {
    pub fn append(&mut self, result: &Poll<Option<Vec<Slot>>>) -> bool {
        match result {
            Poll::Pending => {
                if let Some(Record::Waiting(count)) = self.record.last_mut() {
                    let Some(next) = count.checked_add(1) else {
                        return false;
                    };
                    *count = next;
                    return true;
                }
                if self.retained >= 4096 {
                    return false;
                }
                self.record.push(Record::Waiting(1));
                self.retained += 1;
            }
            Poll::Ready(Some(binding)) => {
                let size = 1 + binding
                    .iter()
                    .map(|slot| slot.token.len() + 2)
                    .sum::<usize>();
                if size > 4096 - self.retained {
                    return false;
                }
                self.record.push(Record::Binding(binding.clone()));
                self.retained += size;
            }
            Poll::Ready(None) => {}
        }
        true
    }
}

#[derive(Default)]
pub(super) struct Playback {
    cursor: usize,
    offset: usize,
    pub progress: usize,
}

impl Playback {
    pub fn step(&mut self, transcript: &Transcript) -> Poll<Option<Vec<Slot>>> {
        let Some(record) = transcript.record.get(self.cursor) else {
            return Poll::Ready(None);
        };
        self.progress += 1;
        match record {
            Record::Waiting(count) => {
                self.offset += 1;
                if self.offset == *count {
                    self.offset = 0;
                    self.cursor += 1;
                }
                Poll::Pending
            }
            Record::Binding(binding) => {
                self.cursor += 1;
                Poll::Ready(Some(binding.clone()))
            }
        }
    }
}
