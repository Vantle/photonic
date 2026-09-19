use crate::matching::Term;
use std::sync::Arc;

pub(crate) struct Selection {
    pub pattern: Arc<Vec<Vec<Term>>>,
    pub order: Vec<usize>,
    pub candidate: Vec<Vec<usize>>,
}

impl Selection {
    pub(crate) fn new(
        pattern: Arc<Vec<Vec<Term>>>,
        index: &crate::index::Index,
        frame: usize,
    ) -> Self {
        let mut candidate = Vec::with_capacity(pattern.len());
        for particle in pattern.iter() {
            let selected = index.candidate(particle, frame);
            if selected.is_empty() {
                candidate.clear();
                break;
            }
            candidate.push(selected);
        }
        if !crate::assignment::feasible(&candidate) {
            candidate.clear();
        }
        let mut order = (0..pattern.len()).collect::<Vec<_>>();
        if !candidate.is_empty() {
            order.sort_by_key(|&position| candidate[position].len());
            candidate = order
                .iter()
                .map(|&position| candidate[position].clone())
                .collect();
        }
        Self {
            pattern,
            order,
            candidate: candidate
                .into_iter()
                .map(|candidate| {
                    candidate
                        .into_iter()
                        .map(|world| index.site(world))
                        .collect()
                })
                .collect(),
        }
    }

    pub(crate) fn retained(&self) -> usize {
        self.pattern.iter().map(Vec::len).sum::<usize>()
            + self.order.len()
            + self.candidate.iter().map(Vec::len).sum::<usize>()
            + self.candidate.len()
    }
}
