use crate::program::Symbol;
use crate::state::State;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
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
        binding: Arc<Vec<Slot>>,
        position: usize,
        cursor: usize,
        end: usize,
    },
}

pub struct Gate {
    pattern: Vec<Vec<Term>>,
    candidate: Vec<Vec<Slot>>,
    prefix: Vec<Vec<Arc<Vec<Slot>>>>,
    seen: Vec<HashSet<Vec<Slot>>>,
    agenda: VecDeque<Task>,
}

impl Gate {
    pub fn new(pattern: Vec<Vec<Term>>) -> Self {
        let count = pattern.len();
        let mut prefix = vec![Vec::new(); count + 1];
        prefix[0].push(Arc::new(Vec::new()));
        Self {
            pattern,
            candidate: vec![Vec::new(); count],
            prefix,
            seen: vec![HashSet::new(); count + 1],
            agenda: VecDeque::new(),
        }
    }

    pub(crate) fn enqueue(&mut self, slot: Slot) {
        let position = slot.position;
        if self.candidate[position].contains(&slot) {
            return;
        }
        self.candidate[position].push(slot.clone());
        self.agenda.push_back(Task::Arrival {
            slot,
            cursor: 0,
            end: self.prefix[position].len(),
        });
    }

    pub(crate) fn pending(&self) -> bool {
        !self.agenda.is_empty()
    }

    pub(crate) fn step(&mut self) -> Option<Vec<Slot>> {
        let (binding, slot) = match self.agenda.pop_front()? {
            Task::Arrival { slot, cursor, end } => {
                if cursor == end {
                    return None;
                }
                let binding = self.prefix[slot.position][cursor].clone();
                self.agenda.push_back(Task::Arrival {
                    slot: slot.clone(),
                    cursor: cursor + 1,
                    end,
                });
                (binding, slot)
            }
            Task::Follow {
                binding,
                position,
                cursor,
                end,
            } => {
                if cursor == end {
                    return None;
                }
                let slot = self.candidate[position][cursor].clone();
                self.agenda.push_back(Task::Follow {
                    binding: binding.clone(),
                    position,
                    cursor: cursor + 1,
                    end,
                });
                (binding, slot)
            }
        };
        if binding.iter().any(|item| item.world == slot.world) {
            return None;
        }
        if binding
            .iter()
            .rev()
            .find(|item| self.pattern[item.position] == self.pattern[slot.position])
            .is_some_and(|item| item.world >= slot.world)
        {
            return None;
        }
        let next = slot.position + 1;
        let mut value = (*binding).clone();
        value.push(slot);
        if !self.seen[next].insert(value.clone()) {
            return None;
        }
        let binding = Arc::new(value);
        self.prefix[next].push(binding.clone());
        if next == self.pattern.len() {
            return Some((*binding).clone());
        }
        self.agenda.push_back(Task::Follow {
            binding,
            position: next,
            cursor: 0,
            end: self.candidate[next].len(),
        });
        None
    }

    pub(crate) fn retained(&self) -> usize {
        self.candidate.iter().map(Vec::len).sum::<usize>()
            + self.prefix.iter().map(Vec::len).sum::<usize>()
            + self.agenda.len()
    }

    pub fn arrive(&mut self, slot: Slot) -> Vec<Vec<Slot>> {
        self.enqueue(slot);
        let mut result = Vec::new();
        while self.pending() {
            result.extend(self.step());
        }
        result
    }
}

pub fn world(pattern: &[Vec<Term>], state: &State, frame: usize) -> Vec<Vec<Slot>> {
    let mut search = crate::search::Search::new(pattern.to_vec(), Arc::new(state.clone()), frame);
    let mut result = Vec::new();
    loop {
        match search.step() {
            std::task::Poll::Pending => {}
            std::task::Poll::Ready(Some(binding)) => result.push(binding),
            std::task::Poll::Ready(None) => return result,
        }
    }
}
