use crate::factor::Budget;
use crate::slot::Slot;
use std::sync::Arc;
use std::task::Poll;

pub(super) enum Record {
    Waiting(usize),
    Binding(Vec<Selection>),
}

pub(super) struct Selection {
    pub site: usize,
    pub token: Vec<usize>,
}

pub(super) struct Trace {
    budget: Arc<Budget>,
    pub record: Vec<Record>,
    pub retained: usize,
    pub complete: bool,
}

impl Trace {
    pub fn new(budget: Arc<Budget>) -> Option<Self> {
        budget.reserve(1).then(|| Self {
            budget,
            record: Vec::new(),
            retained: 1,
            complete: false,
        })
    }

    fn reserve(&mut self, size: usize) -> bool {
        if size > 4096 - self.retained || !self.budget.reserve(size) {
            return false;
        }
        self.retained += size;
        true
    }

    pub fn append(&mut self, result: &Poll<Option<Vec<Slot>>>) -> bool {
        match result {
            Poll::Ready(None) => self.complete = true,
            Poll::Pending => {
                if let Some(Record::Waiting(count)) = self.record.last_mut() {
                    *count += 1;
                } else {
                    if !self.reserve(1) {
                        return false;
                    }
                    self.record.push(Record::Waiting(1));
                }
            }
            Poll::Ready(Some(binding)) => {
                let size = 1 + binding
                    .iter()
                    .map(|slot| slot.token.len() + 1)
                    .sum::<usize>();
                if !self.reserve(size) {
                    return false;
                }
                let binding = binding
                    .iter()
                    .map(|slot| Selection {
                        site: slot.world,
                        token: slot.token.clone(),
                    })
                    .collect();
                self.record.push(Record::Binding(binding));
            }
        }
        true
    }

    #[cfg(test)]
    pub fn size(&self) -> usize {
        1 + self
            .record
            .iter()
            .map(|record| match record {
                Record::Waiting(_) => 1,
                Record::Binding(binding) => {
                    1 + binding
                        .iter()
                        .map(|slot| slot.token.len() + 1)
                        .sum::<usize>()
                }
            })
            .sum::<usize>()
    }
}

impl Drop for Trace {
    fn drop(&mut self) {
        self.budget.release(self.retained);
    }
}
