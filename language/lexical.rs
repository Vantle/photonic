use crate::membership::Set;
use crate::state::State;

pub(crate) struct Index {
    child: Vec<Set>,
    count: usize,
}

impl Index {
    pub fn new(state: &State) -> Self {
        let mut index = Self {
            child: vec![Set::default(); state.frame.len()],
            count: 0,
        };
        for (frame, value) in state.frame.iter().enumerate() {
            if let Some(parent) = value.lexical {
                index.count += usize::from(index.child[parent].insert(frame));
            }
        }
        index
    }

    pub fn update(&mut self, source: &State, state: &State, changed: &[usize]) {
        self.child
            .resize_with(self.child.len().max(state.frame.len()), Set::default);
        for &frame in changed {
            if let Some(parent) = source.frame.get(frame).and_then(|value| value.lexical) {
                self.count -= usize::from(self.child[parent].remove(&frame));
            }
        }
        for &frame in changed {
            if let Some(parent) = state.frame.get(frame).and_then(|value| value.lexical) {
                self.count += usize::from(self.child[parent].insert(frame));
            }
        }
        self.child.truncate(state.frame.len());
    }

    pub fn select(&self, changed: &[usize]) -> Vec<usize> {
        let mut pending = changed.to_vec();
        let mut selected = Set::default();
        while let Some(frame) = pending.pop() {
            if !selected.insert(frame) {
                continue;
            }
            if let Some(child) = self.child.get(frame) {
                pending.extend(child.iter().copied());
            }
        }
        selected.iter().copied().collect()
    }

    pub fn retained(&self) -> usize {
        self.child.len() + self.count
    }
}

#[cfg(test)]
#[path = "test/context.rs"]
mod test;
