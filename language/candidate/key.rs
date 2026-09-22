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
        index.affected.get(&self.frame).is_some_and(|symbol| {
            self.pattern.iter().all(|term| {
                symbol.contains(&term.value) || index.visible(self.frame, term).next().is_some()
            })
        })
    }

    pub fn insertion(&self, index: &Index) -> Vec<usize> {
        index
            .insertion
            .iter()
            .copied()
            .filter(|&site| {
                let location = index.location(site);
                location.frame(&index.state) == self.frame
                    && (!self.pattern.is_empty() || location.world().is_some())
                    && self
                        .pattern
                        .iter()
                        .all(|term| index.quantity(term, self.frame, site) > 0)
            })
            .collect()
    }

    pub fn retained(&self) -> usize {
        self.pattern.len() + 1
    }
}
