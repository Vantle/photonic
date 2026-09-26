use super::binding::{Binding, Selection};
use super::slot::Slot;
use crate::budget::{Account, Reservation};
use std::sync::Arc;
use std::task::Poll;

pub(super) const CAPACITY: usize = 4096;
pub(super) const LENGTH: usize = 65_536;

#[derive(Clone)]
pub(super) enum Record {
    Waiting(usize),
    Binding(Binding),
}

pub(super) struct Trace {
    reservation: Reservation,
    header: u16,
    binding: u16,
    pub record: super::transcript::Transcript,
    pub length: usize,
    pub complete: bool,
}

impl Trace {
    pub fn new(budget: &Account, header: usize) -> Option<Self> {
        if header > CAPACITY {
            return None;
        }
        Some(Self {
            reservation: budget.reserve(header)?,
            header: header.try_into().unwrap(),
            binding: 0,
            record: super::transcript::Transcript::default(),
            length: 0,
            complete: false,
        })
    }

    #[inline]
    pub fn retained(&self) -> usize {
        self.reservation.size()
    }

    fn reserve(&mut self, size: usize, allowance: usize) -> bool {
        size <= allowance && size <= CAPACITY - self.retained() && self.reservation.grow(size)
    }

    pub fn append(
        &mut self,
        result: &Poll<Option<Vec<Slot>>>,
        projection: std::ops::RangeFrom<usize>,
        allowance: usize,
    ) -> bool {
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
                let binding = &binding[projection];
                let size = 1 + binding
                    .iter()
                    .map(|slot| slot.token.len() + 1)
                    .sum::<usize>();
                if !self.reserve(size, allowance) {
                    return false;
                }
                self.record.push(Record::Binding(Binding::new(binding)));
                self.binding += 1;
            }
        }
        if !matches!(result, Poll::Ready(None)) {
            self.length += 1;
        }
        true
    }

    #[inline]
    pub fn duplicate(&self, allowance: usize) -> Option<Self> {
        if self.retained() > allowance {
            return None;
        }
        Some(Self {
            reservation: self.reservation.duplicate()?,
            header: self.header,
            binding: self.binding,
            record: self.record.clone(),
            length: self.length,
            complete: self.complete,
        })
    }

    pub fn extend(&mut self, trace: &Self, prefix: &[Slot], allowance: usize) -> bool {
        let Some(length) = self
            .length
            .checked_add(trace.length)
            .filter(|&length| length <= LENGTH)
        else {
            return false;
        };
        let merged = matches!(
            (self.record.last(), trace.record.first()),
            (Some(Record::Waiting(_)), Some(Record::Waiting(_)))
        );
        let inherited = prefix
            .iter()
            .map(|slot| slot.token.len() + 1)
            .sum::<usize>();
        let retained = trace.retained() - usize::from(trace.header)
            + usize::from(trace.binding) * (inherited + usize::from(!prefix.is_empty()))
            - usize::from(merged);
        if !self.reserve(retained, allowance) {
            return false;
        }
        let prefix = (!prefix.is_empty() && trace.binding != 0).then(|| {
            prefix
                .iter()
                .map(|slot| Selection {
                    site: slot.site,
                    token: slot.token.clone(),
                })
                .collect::<Arc<[Selection]>>()
        });
        let mut record = trace.record.iter();
        if merged
            && let Some(Record::Waiting(count)) = record.next()
            && let Some(Record::Waiting(previous)) = self.record.last_mut()
        {
            *previous += count;
        }
        self.record.extend(record.map(|record| {
            match record {
                Record::Waiting(count) => Record::Waiting(*count),
                Record::Binding(binding) => Record::Binding(
                    prefix
                        .as_ref()
                        .map_or_else(|| binding.clone(), |prefix| binding.prepend(prefix.clone())),
                ),
            }
        }));
        self.length = length;
        self.binding += trace.binding;
        true
    }

    #[cfg(test)]
    pub fn size(&self) -> usize {
        usize::from(self.header)
            + self
                .record
                .iter()
                .map(|record| match record {
                    Record::Waiting(_) => 1,
                    Record::Binding(binding) => binding.size(),
                })
                .sum::<usize>()
    }
}

#[cfg(test)]
#[path = "../test/trace.rs"]
mod test;
