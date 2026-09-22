use smallvec::SmallVec;
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

impl<Value: Ord> Set<Value> {
    pub(crate) fn contains(&self, value: &Value) -> bool {
        self.slice().binary_search(value).is_ok()
    }

    pub(crate) fn union<'set>(&'set self, other: &'set Self) -> Union<'set, Value> {
        Union {
            left: self.slice(),
            right: other.slice(),
        }
    }
}

pub(crate) struct Union<'set, Value> {
    left: &'set [Value],
    right: &'set [Value],
}

impl<'set, Value: Ord> Iterator for Union<'set, Value> {
    type Item = &'set Value;

    fn next(&mut self) -> Option<Self::Item> {
        let Some(left) = self.left.first() else {
            let (value, rest) = self.right.split_first()?;
            self.right = rest;
            return Some(value);
        };
        let Some(right) = self.right.first() else {
            self.left = &self.left[1..];
            return Some(left);
        };
        if left <= right {
            self.left = &self.left[1..];
        }
        if right <= left {
            self.right = &self.right[1..];
        }
        Some(left.min(right))
    }
}

impl<Value> Default for Set<Value> {
    fn default() -> Self {
        Self(Storage::Empty)
    }
}

impl<Value: Clone + Ord> FromIterator<Value> for Set<Value> {
    fn from_iter<Input: IntoIterator<Item = Value>>(value: Input) -> Self {
        let mut input = value.into_iter();
        let Some(first) = input.next() else {
            return Self::default();
        };
        let Some(second) = input.next() else {
            return Self::single(first);
        };
        let mut value = [first, second]
            .into_iter()
            .chain(input)
            .collect::<SmallVec<[Value; 8]>>();
        value.sort_unstable();
        value.dedup();
        if value.len() == 1 {
            return Self::single(value.pop().unwrap());
        }
        Self(Storage::Shared(Arc::from(value.as_slice())))
    }
}

impl<Value: Clone + Ord> From<BTreeSet<Value>> for Set<Value> {
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
