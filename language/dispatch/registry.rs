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

    pub fn values(&self) -> impl Iterator<Item = &usize> {
        self.frame.iter().flat_map(BTreeMap::values)
    }

    pub fn range(&self, interval: RangeInclusive<Key>) -> impl Iterator<Item = (Key, &usize)> {
        assert_eq!(interval.start().frame, interval.end().frame);
        let frame = interval.start().frame;
        let interval = Position::from(interval.start())..=Position::from(interval.end());
        self.frame
            .get(frame)
            .into_iter()
            .flat_map(move |entry| entry.range(interval.clone()))
            .map(move |(key, value)| (key.key(frame), value))
    }

    pub fn extract(
        &mut self,
        interval: RangeInclusive<Key>,
        mut predicate: impl FnMut(&Key, &mut usize) -> bool,
    ) -> impl Iterator<Item = (Key, usize)> {
        assert_eq!(interval.start().frame, interval.end().frame);
        let frame = interval.start().frame;
        let interval = Position::from(interval.start())..=Position::from(interval.end());
        let count = &mut self.count;
        self.frame[frame]
            .extract_if(interval, move |key, value| {
                predicate(&key.key(frame), value)
            })
            .map(move |(key, value)| (key.key(frame), value))
            .inspect(move |_| *count -= 1)
    }
}

#[cfg(test)]
#[path = "../test/registry.rs"]
mod test;
