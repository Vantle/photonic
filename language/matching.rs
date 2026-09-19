use crate::program::Symbol;
use indexmap::IndexSet;
use std::collections::VecDeque;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Term {
    pub value: Symbol,
    pub capture: Option<usize>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Slot {
    pub world: usize,
    pub token: Vec<usize>,
    pub position: usize,
}

pub fn pattern(input: &[Vec<Symbol>], capture: Option<usize>) -> Vec<Vec<Term>> {
    input
        .iter()
        .map(|particle| {
            particle
                .iter()
                .map(|&value| Term {
                    value,
                    capture: capture.filter(|_| matches!(value, Symbol::Rule(_))),
                })
                .collect()
        })
        .collect()
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

pub struct Gate {
    pattern: Vec<Vec<Term>>,
    candidate: Vec<IndexSet<Slot>>,
    prefix: Vec<Vec<Option<usize>>>,
    binding: Vec<crate::prefix::Prefix>,
    agenda: VecDeque<Task>,
}

impl Gate {
    pub fn new(pattern: Vec<Vec<Term>>) -> Self {
        let count = pattern.len();
        let mut prefix = vec![Vec::new(); count + 1];
        prefix[0].push(None);
        Self {
            pattern,
            candidate: vec![IndexSet::new(); count],
            prefix,
            binding: Vec::new(),
            agenda: VecDeque::new(),
        }
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
        let mut cursor = binding;
        let mut equivalent = false;
        while let Some(index) = cursor {
            let prefix = &self.binding[index];
            if prefix.slot.world == slot.world {
                return None;
            }
            if !equivalent && self.pattern[prefix.slot.position] == self.pattern[slot.position] {
                if prefix.slot.world >= slot.world {
                    return None;
                }
                equivalent = true;
            }
            cursor = prefix.parent;
        }
        let next = slot.position + 1;
        if next == self.pattern.len() {
            let mut value = Vec::with_capacity(next);
            value.push(slot);
            let mut cursor = binding;
            while let Some(index) = cursor {
                let prefix = &self.binding[index];
                value.push(prefix.slot.clone());
                cursor = prefix.parent;
            }
            value.reverse();
            return Some(value);
        }
        let index = self.binding.len();
        self.binding.push(crate::prefix::Prefix {
            parent: binding,
            slot,
        });
        let binding = Some(index);
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
