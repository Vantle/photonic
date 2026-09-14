#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Map<Key, Value>(Vec<(Key, Value)>);

impl<Key, Value> Map<Key, Value> {
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&Key, &Value)> {
        self.0.iter().map(|(key, value)| (key, value))
    }
}

impl<Key: Ord, Value> FromIterator<(Key, Value)> for Map<Key, Value> {
    fn from_iter<Input: IntoIterator<Item = (Key, Value)>>(value: Input) -> Self {
        let mut value = value.into_iter().collect::<Vec<_>>();
        value.sort_by(|left, right| left.0.cmp(&right.0));
        value.dedup_by(|right, left| {
            if right.0 != left.0 {
                return false;
            }
            std::mem::swap(left, right);
            true
        });
        Self(value)
    }
}

impl<Key: Ord, Value> std::ops::Index<&Key> for Map<Key, Value> {
    type Output = Value;

    fn index(&self, key: &Key) -> &Value {
        let index = self
            .0
            .binary_search_by(|(candidate, _)| candidate.cmp(key))
            .expect("relation contains the key");
        &self.0[index].1
    }
}

impl<Key, Value> IntoIterator for Map<Key, Value> {
    type Item = (Key, Value);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
#[path = "test/relation.rs"]
mod test;
