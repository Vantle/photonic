use crate::slot::Slot;
use crate::term::Term;
use indexmap::IndexSet;
use std::collections::VecDeque;

mod sharing;
mod store;

pub(crate) use store::Store;

#[derive(Eq, Hash, PartialEq)]
pub(super) struct Constraint {
    group: usize,
    location: crate::location::Location,
}

enum Task {
    Arrival {
        slot: Slot,
        cursor: usize,
        end: usize,
    },
    Follow {
        binding: Option<usize>,
        position: usize,
        cursor: usize,
        end: usize,
    },
}

pub(crate) struct Gate {
    shared: Option<sharing::Sharing>,
    group: Vec<usize>,
    candidate: Vec<IndexSet<Slot, crate::hashing::Builder>>,
    prefix: Vec<Vec<Option<usize>>>,
    binding: crate::prefix::Arena,
    agenda: VecDeque<Task>,
}

impl Gate {
    pub fn new<Value: AsRef<[Term]>>(pattern: impl IntoIterator<Item = Value>) -> Self {
        let pattern = pattern.into_iter().collect::<smallvec::SmallVec<[_; 2]>>();
        let pattern = pattern
            .iter()
            .map(|value| value.as_ref())
            .collect::<smallvec::SmallVec<[_; 2]>>();
        let count = pattern.len();
        let mut prefix = vec![Vec::new(); count + 1];
        prefix[0].push(None);
        Self {
            shared: None,
            group: crate::partition::classify(&pattern),
            candidate: vec![IndexSet::default(); count],
            prefix,
            binding: crate::prefix::Arena::default(),
            agenda: VecDeque::new(),
        }
    }

    pub(crate) fn shared<Value: AsRef<[Term]>>(
        pattern: impl IntoIterator<Item = Value>,
        store: &std::sync::Arc<Store>,
    ) -> Self {
        let mut gate = Self::new(pattern);
        if Store::eligible(gate.group.len()) {
            gate.shared = Some(sharing::Sharing::new(store.clone()));
        }
        gate
    }

    pub(crate) fn evict(&mut self) {
        self.shared = None;
    }

    fn accepts(&self, binding: Option<usize>, slot: &Slot) -> bool {
        let mut equivalent = false;
        for prefix in self.binding.iter(binding) {
            if prefix.location == slot.location {
                return false;
            }
            if !equivalent && self.group[prefix.position] == self.group[slot.position] {
                if prefix.location >= slot.location {
                    return false;
                }
                equivalent = true;
            }
        }
        true
    }

    pub(crate) fn enqueue(&mut self, slot: Slot) {
        let position = slot.position;
        if !self.candidate[position].insert(slot.clone()) {
            return;
        }
        let end = self.prefix[position].len();
        if end > 0 {
            self.agenda.push_back(Task::Arrival {
                slot,
                cursor: 0,
                end,
            });
        }
    }

    pub(crate) fn pending(&self) -> bool {
        !self.agenda.is_empty()
    }

    pub(crate) fn step(&mut self) -> Option<Vec<Slot>> {
        let (binding, slot) = match self.agenda.pop_front()? {
            Task::Arrival { slot, cursor, end } => {
                let binding = self.prefix[slot.position][cursor];
                if cursor + 1 < end {
                    self.agenda.push_back(Task::Arrival {
                        slot: slot.clone(),
                        cursor: cursor + 1,
                        end,
                    });
                }
                (binding, slot)
            }
            Task::Follow {
                binding,
                position,
                cursor,
                end,
            } => {
                let slot = self.candidate[position][cursor].clone();
                if cursor + 1 < end {
                    self.agenda.push_back(Task::Follow {
                        binding,
                        position,
                        cursor: cursor + 1,
                        end,
                    });
                }
                (binding, slot)
            }
        };
        let decision = self.shared.as_ref().map_or_else(
            || store::Decision {
                accepted: self.accepts(binding, &slot),
                identity: None,
            },
            |shared| {
                shared.select(
                    binding,
                    Constraint {
                        group: self.group[slot.position],
                        location: slot.location,
                    },
                    || self.accepts(binding, &slot),
                )
            },
        );
        if !decision.accepted {
            return None;
        }
        let next = slot.position + 1;
        if next == self.group.len() {
            return Some(self.binding.complete(binding, slot));
        }
        let binding = Some(self.binding.push(binding, slot));
        if let Some(shared) = &mut self.shared {
            shared.push(decision.identity);
        }
        self.prefix[next].push(binding);
        let end = self.candidate[next].len();
        if end > 0 {
            self.agenda.push_back(Task::Follow {
                binding,
                position: next,
                cursor: 0,
                end,
            });
        }
        None
    }

    pub(crate) fn retained(&self) -> usize {
        self.candidate.iter().map(IndexSet::len).sum::<usize>()
            + self.prefix.iter().map(Vec::len).sum::<usize>()
            + self.agenda.len()
            + self.shared.as_ref().map_or(0, sharing::Sharing::retained)
    }

    #[cfg(test)]
    pub fn arrive(&mut self, slot: Slot) -> Vec<Vec<Slot>> {
        self.enqueue(slot);
        let mut result = Vec::new();
        while self.pending() {
            result.extend(self.step());
        }
        result
    }
}

#[cfg(test)]
#[path = "test/gate.rs"]
mod test;
