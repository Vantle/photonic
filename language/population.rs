use crate::state::Token;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

mod selection;

use selection::Selection;

#[derive(Clone, Default)]
pub(crate) struct Set {
    value: Option<Arc<[Token]>>,
    selection: Option<Arc<Selection>>,
    capture: Option<usize>,
}

pub(crate) struct Entry<'set> {
    value: &'set [Token],
    selection: Option<&'set Selection>,
    position: usize,
    word: u64,
    remaining: usize,
}

pub(crate) struct Difference<'set> {
    value: &'set Set,
    excluded: Option<&'set Set>,
    width: usize,
    position: usize,
    word: u64,
}

impl Set {
    pub fn len(&self) -> usize {
        self.selection.as_ref().map_or_else(
            || self.value.as_ref().map_or(0, |value| value.len()),
            |value| value.count,
        )
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn entry(&self) -> Entry<'_> {
        Entry {
            value: self.value.as_ref().map_or(&[], |value| value.as_ref()),
            selection: self.selection.as_deref(),
            position: 0,
            word: 0,
            remaining: self.len(),
        }
    }

    pub fn iter(&self) -> Traversal<'_> {
        Traversal(self.entry())
    }

    pub fn at(&self, position: usize) -> &Token {
        &self.value.as_ref().unwrap()[position]
    }

    pub fn capture(&self) -> Option<usize> {
        self.capture
    }

    pub fn retain(&mut self, mut selected: impl FnMut(&Token) -> bool) {
        let removal = self
            .entry()
            .filter_map(|(position, value)| (!selected(value)).then_some(position))
            .collect::<Vec<_>>();
        if removal.is_empty() {
            return;
        }
        if removal.len() == self.len() {
            self.clear();
            return;
        }
        let selection = self
            .selection
            .get_or_insert_with(|| Arc::new(Selection::new(self.value.as_ref().unwrap().len())));
        let selection = Arc::make_mut(selection);
        for position in removal {
            selection.remove(position);
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn shared(&self, other: &Self) -> bool {
        match (&self.value, &other.value) {
            (Some(left), Some(right)) => Arc::ptr_eq(left, right),
            (None, None) => true,
            _ => false,
        }
    }

    #[inline]
    pub fn difference<'set>(&'set self, next: &'set Self) -> (Difference<'set>, Difference<'set>) {
        let shared = self.shared(next);
        (
            Difference::new(self, next, shared),
            Difference::new(next, self, shared),
        )
    }

    #[inline]
    fn word(&self, position: usize) -> u64 {
        self.selection
            .as_ref()
            .map_or(u64::MAX, |selection| selection.word[position])
    }
}

impl<'set> Difference<'set> {
    #[inline]
    fn new(value: &'set Set, other: &'set Set, shared: bool) -> Self {
        Self {
            value,
            excluded: shared.then_some(other),
            width: value.value.as_ref().map_or(0, |value| value.len()),
            position: 0,
            word: 0,
        }
    }
}

impl Iterator for Difference<'_> {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.word == 0 {
            let offset = self.position * 64;
            if offset >= self.width {
                return None;
            }
            let excluded = self
                .excluded
                .map_or(0, |excluded| excluded.word(self.position));
            self.word = self.value.word(self.position) & !excluded;
            self.word &= u64::MAX >> (64 - (self.width - offset).min(64));
            self.position += 1;
        }
        let bit = self.word.trailing_zeros() as usize;
        self.word &= self.word - 1;
        Some((self.position - 1) * 64 + bit)
    }
}

impl<'set> Iterator for Entry<'set> {
    type Item = (usize, &'set Token);
    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        let Some(selection) = self.selection else {
            let position = self.position;
            self.position += 1;
            return Some((position, &self.value[position]));
        };
        while self.word == 0 {
            self.word = selection.word[self.position];
            self.position += 1;
        }
        let position = (self.position - 1) * 64 + self.word.trailing_zeros() as usize;
        self.word &= self.word - 1;
        Some((position, &self.value[position]))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl ExactSizeIterator for Entry<'_> {}

impl From<Vec<Token>> for Set {
    fn from(value: Vec<Token>) -> Self {
        let Some(first) = value.first() else {
            return Self::default();
        };
        let capture = first
            .capture
            .filter(|&capture| value.iter().all(|token| token.capture == Some(capture)));
        Self {
            value: Some(value.into()),
            selection: None,
            capture,
        }
    }
}

impl FromIterator<Token> for Set {
    fn from_iter<Source: IntoIterator<Item = Token>>(source: Source) -> Self {
        source.into_iter().collect::<Vec<_>>().into()
    }
}

pub(crate) struct Traversal<'set>(Entry<'set>);

impl<'set> Iterator for Traversal<'set> {
    type Item = &'set Token;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next().map(|(_, value)| value)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl ExactSizeIterator for Traversal<'_> {}

impl<'set> IntoIterator for &'set Set {
    type Item = &'set Token;
    type IntoIter = Traversal<'set>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl PartialEq for Set {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            return false;
        }
        if self.shared(other) && self.selection == other.selection {
            return true;
        }
        self.iter().eq(other.iter())
    }
}

impl Eq for Set {}

impl PartialOrd for Set {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Set {
    fn cmp(&self, other: &Self) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

impl Hash for Set {
    fn hash<Output: Hasher>(&self, output: &mut Output) {
        self.len().hash(output);
        for value in self {
            value.hash(output);
        }
    }
}

impl std::fmt::Debug for Set {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output.debug_list().entries(self.iter()).finish()
    }
}

#[cfg(test)]
#[path = "test/population.rs"]
mod test;
