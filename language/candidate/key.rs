use crate::index::Index;
use crate::term::Term;

#[derive(Eq, Hash, PartialEq)]
pub(super) struct Key {
    frame: usize,
    pattern: Vec<Term>,
}

impl Key {
    pub fn new(pattern: impl IntoIterator<Item = Term>, frame: usize) -> Self {
        let mut pattern = pattern.into_iter().collect::<Vec<_>>();
        pattern.sort_unstable();
        pattern.dedup();
        Self { frame, pattern }
    }

    pub fn select(&self, index: &Index) -> Vec<usize> {
        index.candidate(self.pattern.iter().cloned(), self.frame)
    }

    pub fn affected(&self, index: &Index) -> bool {
        index.affects(self.frame, self.pattern.iter().cloned())
    }

    pub fn insertion(&self, index: &Index) -> Vec<usize> {
        index
            .insertion
            .iter()
            .copied()
            .filter(|&site| index.admits(site, self.frame, self.pattern.iter().cloned()))
            .collect()
    }

    pub fn retained(&self) -> usize {
        self.pattern.len() + 1
    }
}
