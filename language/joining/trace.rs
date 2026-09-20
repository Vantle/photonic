use crate::factor::Budget;
use crate::slot::Slot;
use std::sync::Arc;
use std::task::Poll;

#[derive(Clone)]
pub(super) enum Record {
    Waiting(usize),
    Binding(Vec<Selection>),
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub(super) struct Selection {
    pub site: usize,
    pub token: Vec<usize>,
}

pub(super) struct Trace {
    budget: Arc<Budget>,
    #[cfg(test)]
    header: usize,
    pub record: Vec<Record>,
    pub retained: usize,
    pub length: usize,
    pub complete: bool,
}

impl Trace {
    pub fn new(budget: Arc<Budget>, header: usize) -> Option<Self> {
        (header <= 4096 && budget.reserve(header)).then(|| Self {
            budget,
            #[cfg(test)]
            header,
            record: Vec::new(),
            retained: header,
            length: 0,
            complete: false,
        })
    }

    fn reserve(&mut self, size: usize, allowance: usize) -> bool {
        if size > allowance || size > 4096 - self.retained || !self.budget.reserve(size) {
            return false;
        }
        self.retained += size;
        true
    }

    pub fn append(&mut self, result: &Poll<Option<Vec<Slot>>>, allowance: usize) -> bool {
        match result {
            Poll::Ready(None) => self.complete = true,
            Poll::Pending => {
                if let Some(Record::Waiting(count)) = self.record.last_mut() {
                    *count += 1;
                } else {
                    if !self.reserve(1, allowance) {
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
                if !self.reserve(size, allowance) {
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
        if !matches!(result, Poll::Ready(None)) {
            self.length += 1;
        }
        true
    }

    pub fn duplicate(&self, allowance: usize) -> Option<Self> {
        if self.retained > allowance || !self.budget.reserve(self.retained) {
            return None;
        }
        Some(Self {
            budget: self.budget.clone(),
            #[cfg(test)]
            header: self.header,
            record: self.record.clone(),
            retained: self.retained,
            length: self.length,
            complete: self.complete,
        })
    }

    #[cfg(test)]
    pub fn size(&self) -> usize {
        self.header
            + self
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
