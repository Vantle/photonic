use crate::exploration::{Exploration, Plan};
use crate::failure::{Code, Failure};
use std::collections::VecDeque;
use std::sync::Arc;

pub struct Store {
    capacity: usize,
    entry: VecDeque<Arc<Exploration>>,
}

impl Default for Store {
    fn default() -> Self {
        Self::new(16)
    }
}

impl Store {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entry: VecDeque::new(),
        }
    }

    fn touch(&mut self, position: usize) -> Option<Arc<Exploration>> {
        let entry = self.entry.remove(position)?;
        self.entry.push_back(entry.clone());
        Some(entry)
    }

    pub(crate) fn explore(&mut self, plan: Plan) -> Result<Arc<Exploration>, Failure> {
        if let Some(position) = self.entry.iter().position(|entry| entry.key == plan.key) {
            return self
                .touch(position)
                .ok_or_else(|| Failure::new(Code::Exploration, "the store lost an entry"));
        }
        let exploration = Arc::new(Exploration::new(plan)?);
        if self.entry.len() >= self.capacity {
            self.entry.pop_front();
        }
        self.entry.push_back(exploration.clone());
        Ok(exploration)
    }

    pub(crate) fn find(&mut self, key: &str) -> Result<Arc<Exploration>, Failure> {
        let key = key.trim().trim_start_matches('x');
        let found = self
            .entry
            .iter()
            .enumerate()
            .filter(|(_, entry)| key.len() >= 4 && entry.key.starts_with(key))
            .map(|(position, _)| position)
            .collect::<Vec<_>>();
        match found.as_slice() {
            [position] => self
                .touch(*position)
                .ok_or_else(|| Failure::new(Code::Exploration, "the store lost an entry")),
            [] => Err(Failure::new(
                Code::Exploration,
                format!("no exploration x{key} is held here; send the program again"),
            )),
            _ => Err(Failure::new(
                Code::Exploration,
                format!("x{key} names several explorations; give more of the key"),
            )),
        }
    }
}
