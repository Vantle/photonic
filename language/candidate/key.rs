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
        index
            .affected
            .get(&self.frame)
            .is_some_and(|symbol| self.pattern.iter().all(|term| symbol.contains(&term.value)))
    }

    pub fn insertion(&self, index: &Index) -> Vec<usize> {
        index
            .insertion
            .iter()
            .copied()
            .filter(|&site| {
                let world = &index.state.world[index.world(site)];
                world.frame == self.frame
                    && self.pattern.iter().all(|term| {
                        if world.particle.len() <= 16 {
                            world.particle.iter().any(|token| term.matches(token))
                        } else {
                            index.quantity(term, self.frame, site) > 0
                        }
                    })
            })
            .collect()
    }

    pub fn retained(&self) -> usize {
        self.pattern.len() + 1
    }
}
