use crate::matching::Term;
use std::sync::Arc;

pub(crate) struct Selection {
    pub pattern: Arc<Vec<Vec<Term>>>,
    pub order: Vec<usize>,
    pub candidate: Vec<Vec<usize>>,
    pub viable: bool,
}

impl Selection {
    pub(crate) fn new(
        pattern: Arc<Vec<Vec<Term>>>,
        index: &crate::index::Index,
        frame: usize,
    ) -> Self {
        let candidate = pattern
            .iter()
            .map(|particle| {
                index
                    .candidate(particle, frame)
                    .into_iter()
                    .map(|world| index.site(world))
                    .collect()
            })
            .collect();
        Self::construct(pattern, candidate)
    }

    fn construct(pattern: Arc<Vec<Vec<Term>>>, candidate: Vec<Vec<usize>>) -> Self {
        let viable = candidate.iter().all(|candidate| !candidate.is_empty())
            && crate::assignment::feasible(&candidate);
        let mut order = (0..pattern.len()).collect::<Vec<_>>();
        order.sort_by_key(|&position| candidate[position].len());
        Self {
            pattern,
            order,
            candidate,
            viable,
        }
    }

    pub(crate) fn advance(&self, index: &crate::index::Index, frame: usize) -> Option<Self> {
        let mut candidate = None;
        for (position, pattern) in self.pattern.iter().enumerate() {
            let previous = &self.candidate[position];
            let insertion = index
                .insertion
                .iter()
                .copied()
                .filter(|&site| {
                    let world = &index.state.world[index.world(site)];
                    world.frame == frame
                        && pattern.iter().all(|term| {
                            world.particle.iter().any(|token| {
                                token.value == term.value
                                    && (!matches!(term.value, crate::program::Symbol::Rule(_))
                                        || token.capture == term.capture)
                            })
                        })
                })
                .collect::<Vec<_>>();
            let removed = previous
                .iter()
                .any(|site| index.removal.contains(site) && !insertion.contains(site));
            let inserted = insertion.iter().any(|site| !previous.contains(site));
            if !removed && !inserted {
                continue;
            }
            let candidate = candidate.get_or_insert_with(|| self.candidate.clone());
            candidate[position].retain(|site| !index.removal.contains(site));
            candidate[position].extend(insertion);
        }
        candidate.map(|candidate| Self::construct(self.pattern.clone(), candidate))
    }

    pub(crate) fn retained(&self) -> usize {
        self.pattern.iter().map(Vec::len).sum::<usize>()
            + self.order.len()
            + self.candidate.iter().map(Vec::len).sum::<usize>()
            + self.candidate.len()
    }
}
