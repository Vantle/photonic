use crate::term::Term;
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

    pub(crate) fn retained(&self) -> usize {
        self.pattern.iter().map(Vec::len).sum::<usize>()
            + self.order.len()
            + self.candidate.iter().map(Vec::len).sum::<usize>()
            + self.candidate.len()
    }
}
