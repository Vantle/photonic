use super::Key;
use std::collections::BTreeMap;
use std::ops::RangeInclusive;

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
struct Position {
    input: usize,
    owner: usize,
}

impl From<&Key> for Position {
    fn from(key: &Key) -> Self {
        Self {
            input: key.input,
            owner: key.owner,
        }
    }
}

impl Position {
    fn key(self, frame: usize) -> Key {
        Key {
            frame,
            input: self.input,
            owner: self.owner,
        }
    }

    fn interval(input: Option<usize>) -> RangeInclusive<Self> {
        let (first, last) = input.map_or((0, usize::MAX), |input| (input, input));
        Self {
            input: first,
            owner: 0,
        }..=Self {
            input: last,
            owner: usize::MAX,
        }
    }
}

pub(super) struct Registry {
    frame: Vec<BTreeMap<Position, usize>>,
    count: usize,
}

impl Registry {
    pub fn new(count: usize) -> Self {
        Self {
            frame: vec![BTreeMap::new(); count],
            count: 0,
        }
    }

    pub fn resize(&mut self, count: usize) {
        self.frame
            .resize_with(self.frame.len().max(count), BTreeMap::new);
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn retained(&self) -> usize {
        self.frame.len()
    }

    pub fn count(&self, frame: usize) -> usize {
        self.frame.get(frame).map_or(0, BTreeMap::len)
    }

    pub fn get(&self, key: &Key) -> Option<&usize> {
        self.frame.get(key.frame)?.get(&Position::from(key))
    }

    pub fn insert(&mut self, key: Key, position: usize) {
        if self.frame[key.frame]
            .insert(Position::from(&key), position)
            .is_none()
        {
            self.count += 1;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Key, &usize)> {
        self.frame.iter().enumerate().flat_map(|(frame, entry)| {
            entry
                .iter()
                .map(move |(key, value)| (key.key(frame), value))
        })
    }

    pub fn value(&self) -> impl Iterator<Item = &usize> {
        self.frame.iter().flat_map(BTreeMap::values)
    }

    pub fn range(&self, frame: usize, input: Option<usize>) -> impl Iterator<Item = (Key, &usize)> {
        let interval = Position::interval(input);
        self.frame
            .get(frame)
            .into_iter()
            .flat_map(move |entry| entry.range(interval.clone()))
            .map(move |(key, value)| (key.key(frame), value))
    }

    pub fn extract(
        &mut self,
        frame: usize,
        input: Option<usize>,
        mut predicate: impl FnMut(&Key, &mut usize) -> bool,
    ) -> impl Iterator<Item = (Key, usize)> {
        let count = &mut self.count;
        self.frame[frame]
            .extract_if(Position::interval(input), move |key, value| {
                predicate(&key.key(frame), value)
            })
            .map(move |(key, value)| (key.key(frame), value))
            .inspect(move |_| *count -= 1)
    }
}

#[cfg(test)]
#[path = "../test/registry.rs"]
mod test;
