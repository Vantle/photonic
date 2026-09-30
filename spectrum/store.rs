use crate::exploration::{Identity, Plan};
use crate::explored::Explored;
use crate::failure::{Code, Failure};
use std::collections::VecDeque;

// Explorations are held by their size, occurrences and events, because one exploration can
// outweigh thousands of small ones; the newest is always kept.
const CAPACITY: usize = 2_000_000;

// A key is 16 hexadecimal digits, and any prefix of at least four names the explorations it starts.
const SHORTEST: usize = 4;
const LONGEST: usize = 16;

// An exploration, the identity its key hashes, and its size, weighed once when it is stored, since
// weighing walks every configuration it holds.
struct Entry {
    explored: Explored,
    identity: Identity,
    size: usize,
}

#[derive(Default)]
pub struct Store {
    entry: VecDeque<Entry>,
    weight: usize,
}

impl Store {
    fn touch(&mut self, position: usize) -> Explored {
        let entry = self
            .entry
            .remove(position)
            .expect("a found position holds an entry");
        let explored = entry.explored.clone();
        self.entry.push_back(entry);
        explored
    }

    // Two identities can share a key, so a hit needs the same identity, and a plan whose key another
    // identity holds is explored on its own; its key then names both, and a lookup by it refuses.
    pub(crate) fn explore(&mut self, plan: Plan) -> Result<Explored, Failure> {
        if let Some(position) = self
            .entry
            .iter()
            .position(|entry| entry.explored.key() == plan.key && entry.identity == plan.identity)
        {
            return Ok(self.touch(position));
        }
        let identity = plan.identity.clone();
        let explored = Explored::new(plan)?;
        let size = explored.size();
        self.weight += size;
        self.entry.push_back(Entry {
            explored: explored.clone(),
            identity,
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
        let text = key.trim();
        let digit = text
            .strip_prefix(['x', 'X'])
            .unwrap_or(text)
            .to_ascii_lowercase();
        if digit.len() < SHORTEST {
            return Err(Failure::new(
                Code::Exploration,
                format!("x{digit} is too short; give at least four digits of the key"),
            ));
        }
        if digit.len() > LONGEST || !digit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(Failure::new(
                Code::Exploration,
                format!(
                    "{text} is not an exploration key; a key is x and 16 hexadecimal digits, such as x91c7f6619ec81b4b"
                ),
            ));
        }
        let found = self
            .entry
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.explored.key().starts_with(&digit))
            .map(|(position, _)| position)
            .collect::<Vec<_>>();
        match found.as_slice() {
            [position] => Ok(self.touch(*position)),
            [] => Err(Failure::new(
                Code::Exploration,
                format!("no exploration x{digit} is held here; send the program again"),
            )),
            [first, rest @ ..]
                if rest.iter().all(|&other| {
                    self.entry[other].explored.key() == self.entry[*first].explored.key()
                }) =>
            {
                Err(Failure::new(
                    Code::Exploration,
                    format!(
                        "x{} names explorations of different programs that share it; send the program again",
                        self.entry[*first].explored.key()
                    ),
                ))
            }
            _ => Err(Failure::new(
                Code::Exploration,
                format!("x{digit} names several explorations; give more of the key"),
            )),
        }
    }
}
