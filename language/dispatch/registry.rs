use super::Key;
use std::collections::BTreeMap;
use std::ops::RangeInclusive;

pub(super) struct Registry {
    frame: Vec<BTreeMap<Key, usize>>,
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
        self.frame.get(key.frame)?.get(key)
    }

    pub fn insert(&mut self, key: Key, position: usize) {
        if self.frame[key.frame].insert(key, position).is_none() {
            self.count += 1;
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Key, &usize)> {
        self.frame.iter().flat_map(BTreeMap::iter)
    }

    pub fn values(&self) -> impl Iterator<Item = &usize> {
        self.frame.iter().flat_map(BTreeMap::values)
    }

    pub fn range(&self, interval: RangeInclusive<Key>) -> impl Iterator<Item = (&Key, &usize)> {
        assert_eq!(interval.start().frame, interval.end().frame);
        self.frame
            .get(interval.start().frame)
            .into_iter()
            .flat_map(move |frame| frame.range(interval.clone()))
    }

    pub fn extract(
        &mut self,
        interval: RangeInclusive<Key>,
        predicate: impl FnMut(&Key, &mut usize) -> bool,
    ) -> impl Iterator<Item = (Key, usize)> {
        assert_eq!(interval.start().frame, interval.end().frame);
        let count = &mut self.count;
        self.frame[interval.start().frame]
            .extract_if(interval, predicate)
            .inspect(move |_| *count -= 1)
    }
}

#[cfg(test)]
#[path = "../test/registry.rs"]
mod test;
