use crate::exploration::Plan;
use crate::explored::Explored;
use crate::failure::{Code, Failure};
use std::collections::VecDeque;

// Explorations are held by their size, occurrences and events, because one exploration can
// outweigh thousands of small ones; the newest is always kept.
const CAPACITY: usize = 2_000_000;

// An exploration and its size, weighed once when it is stored, since weighing walks every
// configuration it holds.
#[derive(Clone)]
struct Entry {
    explored: Explored,
    size: usize,
}

#[derive(Default)]
pub struct Store {
    entry: VecDeque<Entry>,
    weight: usize,
}

impl Store {
    fn touch(&mut self, position: usize) -> Explored {
        let entry = self.entry[position].clone();
        self.entry.remove(position);
        self.entry.push_back(entry.clone());
        entry.explored
    }

    pub(crate) fn explore(&mut self, plan: Plan) -> Result<Explored, Failure> {
        if let Some(position) = self
            .entry
            .iter()
            .position(|entry| entry.explored.key() == plan.key)
        {
            return Ok(self.touch(position));
        }
        let explored = Explored::new(plan)?;
        let size = explored.size();
        self.weight += size;
        self.entry.push_back(Entry {
            explored: explored.clone(),
            size,
        });
        while self.entry.len() > 1
            && self.weight > CAPACITY
            && let Some(evicted) = self.entry.pop_front()
        {
            self.weight -= evicted.size;
        }
        Ok(explored)
    }

    pub(crate) fn find(&mut self, key: &str) -> Result<Explored, Failure> {
        let key = key.trim().trim_start_matches('x');
        let found = self
            .entry
            .iter()
            .enumerate()
            .filter(|(_, entry)| key.len() >= 4 && entry.explored.key().starts_with(key))
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
