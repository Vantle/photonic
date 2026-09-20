use smallvec::SmallVec;
use std::collections::BTreeSet;

enum Storage {
    Flat(SmallVec<[usize; 2]>),
    Tree(BTreeSet<usize>),
}

pub(crate) struct Set(Storage);

pub(crate) enum Traversal<'set> {
    Flat(std::slice::Iter<'set, usize>),
    Tree(std::collections::btree_set::Iter<'set, usize>),
}

impl Default for Set {
    fn default() -> Self {
        Self(Storage::Flat(SmallVec::new()))
    }
}

impl Set {
    pub fn insert(&mut self, value: usize) {
        match &mut self.0 {
            Storage::Flat(sequence) => {
                let Err(position) = sequence.binary_search(&value) else {
                    return;
                };
                sequence.insert(position, value);
                if sequence.len() > 32 {
                    self.0 = Storage::Tree(sequence.drain(..).collect());
                }
            }
            Storage::Tree(sequence) => {
                sequence.insert(value);
            }
        }
    }

    pub fn remove(&mut self, value: &usize) {
        match &mut self.0 {
            Storage::Flat(sequence) => {
                if let Ok(position) = sequence.binary_search(value) {
                    sequence.remove(position);
                }
            }
            Storage::Tree(sequence) => {
                sequence.remove(value);
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        match &self.0 {
            Storage::Flat(value) => value.is_empty(),
            Storage::Tree(value) => value.is_empty(),
        }
    }

    pub fn iter(&self) -> Traversal<'_> {
        match &self.0 {
            Storage::Flat(value) => Traversal::Flat(value.iter()),
            Storage::Tree(value) => Traversal::Tree(value.iter()),
        }
    }
}

impl<'set> Iterator for Traversal<'set> {
    type Item = &'set usize;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Flat(value) => value.next(),
            Self::Tree(value) => value.next(),
        }
    }
}
