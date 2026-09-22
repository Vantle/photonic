#[derive(Clone, Default)]
pub(crate) struct Set {
    word: Vec<u64>,
    count: usize,
}

pub(crate) struct Traversal<'set> {
    word: std::iter::Enumerate<std::slice::Iter<'set, u64>>,
    offset: usize,
    current: u64,
}

impl Set {
    pub fn insert(&mut self, value: usize) -> bool {
        let position = value / 64;
        if position >= self.word.len() {
            self.word.resize(position + 1, 0);
        }
        let bit = 1 << (value % 64);
        let word = &mut self.word[position];
        if *word & bit != 0 {
            return false;
        }
        *word |= bit;
        self.count += 1;
        true
    }

    pub fn remove(&mut self, value: usize) -> bool {
        let Some(word) = self.word.get_mut(value / 64) else {
            return false;
        };
        let bit = 1 << (value % 64);
        if *word & bit == 0 {
            return false;
        }
        *word &= !bit;
        self.count -= 1;
        true
    }

    pub fn contains(&self, value: usize) -> bool {
        self.word
            .get(value / 64)
            .is_some_and(|word| word & (1 << (value % 64)) != 0)
    }

    pub fn union(&mut self, other: &Self) {
        if other.word.len() > self.word.len() {
            self.word.resize(other.word.len(), 0);
        }
        for (word, other) in self.word.iter_mut().zip(&other.word) {
            *word |= other;
        }
        self.count = self
            .word
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum();
    }

    pub fn clear(&mut self) {
        self.word.fill(0);
        self.count = 0;
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn iter(&self) -> Traversal<'_> {
        Traversal {
            word: self.word.iter().enumerate(),
            offset: 0,
            current: 0,
        }
    }
}

impl Iterator for Traversal<'_> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        while self.current == 0 {
            let (position, &word) = self.word.next()?;
            self.offset = position * 64;
            self.current = word;
        }
        let bit = self.current.trailing_zeros() as usize;
        self.current &= self.current - 1;
        Some(self.offset + bit)
    }
}

impl<'set> IntoIterator for &'set Set {
    type Item = usize;
    type IntoIter = Traversal<'set>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl FromIterator<usize> for Set {
    fn from_iter<Value: IntoIterator<Item = usize>>(value: Value) -> Self {
        let mut set = Self::default();
        for value in value {
            set.insert(value);
        }
        set
    }
}

#[cfg(test)]
#[path = "test/mask.rs"]
mod test;
