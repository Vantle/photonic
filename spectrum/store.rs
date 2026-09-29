use crate::exploration::Plan;
use crate::explored::Explored;
use crate::failure::{Code, Failure};
use std::collections::VecDeque;

// Explorations are held by their size, occurrences and events, because one exploration can
// outweigh thousands of small ones; the newest is always kept.
const CAPACITY: usize = 2_000_000;

#[derive(Default)]
pub struct Store {
    entry: VecDeque<Explored>,
}

impl Store {
    fn touch(&mut self, position: usize) -> Explored {
        let entry = self.entry[position].clone();
        self.entry.remove(position);
        self.entry.push_back(entry.clone());
        entry
    }

    pub(crate) fn explore(&mut self, plan: Plan) -> Result<Explored, Failure> {
        if let Some(position) = self.entry.iter().position(|entry| entry.key() == plan.key) {
            return Ok(self.touch(position));
        }
        let explored = Explored::new(plan)?;
        self.entry.push_back(explored.clone());
        while self.entry.len() > 1
            && self.entry.iter().map(Explored::size).sum::<usize>() > CAPACITY
        {
            self.entry.pop_front();
        }
        Ok(explored)
    }

    pub(crate) fn find(&mut self, key: &str) -> Result<Explored, Failure> {
        let key = key.trim().trim_start_matches('x');
        let found = self
            .entry
            .iter()
            .enumerate()
            .filter(|(_, entry)| key.len() >= 4 && entry.key().starts_with(key))
            .map(|(position, _)| position)
            .collect::<Vec<_>>();
        match found.as_slice() {
            [position] => Ok(self.touch(*position)),
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
