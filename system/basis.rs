use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum Storage<Value> {
    Empty,
    Inline(Value),
    Shared(Arc<[Value]>),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Set<Value>(Storage<Value>);

impl<Value> Set<Value> {
    pub(crate) fn single(value: Value) -> Self {
        Self(Storage::Inline(value))
    }

    pub(crate) fn iter(&self) -> std::slice::Iter<'_, Value> {
        self.slice().iter()
    }

    pub(crate) fn len(&self) -> usize {
        self.slice().len()
    }

    pub(crate) fn first(&self) -> Option<&Value> {
        self.slice().first()
    }

    fn slice(&self) -> &[Value] {
        match &self.0 {
            Storage::Empty => &[],
            Storage::Inline(value) => std::slice::from_ref(value),
            Storage::Shared(value) => value,
        }
    }
}

impl<Value> Default for Set<Value> {
    fn default() -> Self {
        Self(Storage::Empty)
    }
}

impl<Value: Ord> FromIterator<Value> for Set<Value> {
    fn from_iter<Input: IntoIterator<Item = Value>>(value: Input) -> Self {
        let mut input = value.into_iter();
        let Some(first) = input.next() else {
            return Self::default();
        };
        let Some(second) = input.next() else {
            return Self::single(first);
        };
        let mut value = [first, second].into_iter().chain(input).collect::<Vec<_>>();
        value.sort_unstable();
        value.dedup();
        if value.len() == 1 {
            return Self::single(value.pop().unwrap());
        }
        Self(Storage::Shared(value.into()))
    }
}

impl<Value: Ord> From<BTreeSet<Value>> for Set<Value> {
    fn from(value: BTreeSet<Value>) -> Self {
        value.into_iter().collect()
    }
}

impl<'set, Value> IntoIterator for &'set Set<Value> {
    type Item = &'set Value;
    type IntoIter = std::slice::Iter<'set, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
#[path = "test/basis.rs"]
mod test;
