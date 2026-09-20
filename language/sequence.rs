use imbl::Vector;
use smallvec::SmallVec;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
enum Storage<Value> {
    Flat(SmallVec<[Value; 2]>),
    Tree(Vector<Value>),
}

#[derive(Clone)]
pub(crate) struct List<Value>(Storage<Value>);

pub(crate) enum Traversal<'list, Value> {
    Flat(std::slice::Iter<'list, Value>),
    Tree(imbl::vector::Iter<'list, Value, imbl::shared_ptr::DefaultSharedPtr>),
}

impl<Value> Default for List<Value> {
    fn default() -> Self {
        Self(Storage::Flat(SmallVec::new()))
    }
}

impl<Value> List<Value> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        match &self.0 {
            Storage::Flat(value) => value.len(),
            Storage::Tree(value) => value.len(),
        }
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, index: usize) -> Option<&Value> {
        match &self.0 {
            Storage::Flat(value) => value.get(index),
            Storage::Tree(value) => value.get(index),
        }
    }

    pub fn iter(&self) -> Traversal<'_, Value> {
        match &self.0 {
            Storage::Flat(value) => Traversal::Flat(value.iter()),
            Storage::Tree(value) => Traversal::Tree(value.iter()),
        }
    }

    pub fn range(&self, range: std::ops::Range<usize>) -> Traversal<'_, Value>
    where
        Value: Clone,
    {
        match &self.0 {
            Storage::Flat(value) => Traversal::Flat(value[range].iter()),
            Storage::Tree(value) => Traversal::Tree(value.focus().narrow(range).into_iter()),
        }
    }
}

impl<Value: Clone> List<Value> {
    pub fn push(&mut self, value: Value) {
        if matches!(&self.0, Storage::Flat(value) if value.len() >= 256) {
            let Storage::Flat(previous) =
                std::mem::replace(&mut self.0, Storage::Flat(SmallVec::new()))
            else {
                unreachable!()
            };
            self.0 = Storage::Tree(previous.into_iter().collect());
        }
        match &mut self.0 {
            Storage::Flat(sequence) => sequence.push(value),
            Storage::Tree(sequence) => sequence.push_back(value),
        }
    }

    pub fn remove(&mut self, index: usize) -> Value {
        match &mut self.0 {
            Storage::Flat(value) => value.remove(index),
            Storage::Tree(value) => value.remove(index),
        }
    }

    pub fn truncate(&mut self, length: usize) {
        match &mut self.0 {
            Storage::Flat(value) => value.truncate(length),
            Storage::Tree(value) => value.truncate(length),
        }
    }

    #[cfg(test)]
    pub fn clear(&mut self) {
        self.truncate(0);
    }

    #[cfg(test)]
    pub fn pop(&mut self) -> Option<Value> {
        if self.is_empty() {
            return None;
        }
        Some(self.remove(self.len() - 1))
    }

    #[cfg(test)]
    pub fn reverse(&mut self) {
        *self = self.iter().rev().cloned().collect();
    }
}

impl<Value: Clone> FromIterator<Value> for List<Value> {
    fn from_iter<Source: IntoIterator<Item = Value>>(source: Source) -> Self {
        let mut list = Self::new();
        list.extend(source);
        list
    }
}

impl<Value: Clone> Extend<Value> for List<Value> {
    fn extend<Source: IntoIterator<Item = Value>>(&mut self, source: Source) {
        for value in source {
            self.push(value);
        }
    }
}

impl<Value: Clone> From<Vec<Value>> for List<Value> {
    fn from(value: Vec<Value>) -> Self {
        value.into_iter().collect()
    }
}

impl<Value> std::ops::Index<usize> for List<Value> {
    type Output = Value;
    fn index(&self, index: usize) -> &Value {
        self.get(index).unwrap()
    }
}

impl<Value: Clone> std::ops::IndexMut<usize> for List<Value> {
    fn index_mut(&mut self, index: usize) -> &mut Value {
        match &mut self.0 {
            Storage::Flat(value) => &mut value[index],
            Storage::Tree(value) => value.get_mut(index).unwrap(),
        }
    }
}

impl<'list, Value> Iterator for Traversal<'list, Value> {
    type Item = &'list Value;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Flat(value) => value.next(),
            Self::Tree(value) => value.next(),
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Flat(value) => value.size_hint(),
            Self::Tree(value) => value.size_hint(),
        }
    }
}

impl<Value> DoubleEndedIterator for Traversal<'_, Value> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            Self::Flat(value) => value.next_back(),
            Self::Tree(value) => value.next_back(),
        }
    }
}

impl<Value> ExactSizeIterator for Traversal<'_, Value> {}

impl<'list, Value> IntoIterator for &'list List<Value> {
    type Item = &'list Value;
    type IntoIter = Traversal<'list, Value>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<Value: PartialEq> PartialEq for List<Value> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}

impl<Value: Eq> Eq for List<Value> {}

impl<Value: PartialOrd> PartialOrd for List<Value> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

impl<Value: Ord> Ord for List<Value> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

impl<Value: Hash> Hash for List<Value> {
    fn hash<Output: Hasher>(&self, output: &mut Output) {
        self.len().hash(output);
        for value in self {
            value.hash(output);
        }
    }
}

impl<Value: std::fmt::Debug> std::fmt::Debug for List<Value> {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output.debug_list().entries(self.iter()).finish()
    }
}
