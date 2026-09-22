use crate::term::Term;
use std::sync::Arc;

mod store;

pub(crate) use store::Store;

pub(crate) struct Selection {
    pub pattern: Arc<Vec<Vec<Term>>>,
    fragment: Vec<std::sync::OnceLock<Arc<crate::pattern::Pattern<Term>>>>,
    shared: Option<Arc<Store>>,
    pub order: Vec<usize>,
    pub candidate: Vec<Vec<usize>>,
    pub viable: bool,
}

impl Selection {
    pub(crate) fn cost(&self, position: usize, world: &crate::state::World) -> usize {
        if self.shared.is_some() || world.particle.len() < 4096 {
            return 0;
        }
        world
            .particle
            .len()
            .saturating_mul(self.pattern[position].len().clamp(1, 8))
    }

    pub(crate) fn new(
        pattern: Arc<Vec<Vec<Term>>>,
        index: &crate::index::Index,
        frame: usize,
    ) -> Self {
        let candidate = pattern
            .iter()
            .map(|particle| index.candidate(particle.iter().cloned(), frame))
            .collect();
        Self::construct(pattern, candidate)
    }

    fn construct(pattern: Arc<Vec<Vec<Term>>>, candidate: Vec<Vec<usize>>) -> Self {
        let viable = candidate.iter().all(|candidate| !candidate.is_empty())
            && crate::assignment::feasible(&candidate);
        let mut order = (0..pattern.len()).collect::<Vec<_>>();
        order.sort_by_key(|&position| candidate[position].len());
        Self {
            shared: None,
            fragment: if pattern.iter().any(|particle| particle.len() >= 8) {
                (0..pattern.len())
                    .map(|_| std::sync::OnceLock::new())
                    .collect()
            } else {
                Vec::new()
            },
            pattern,
            order,
            candidate,
            viable,
        }
    }

    pub(crate) fn gate(&self) -> Option<crate::gate::Gate> {
        if self.pattern.len() <= 1 {
            return None;
        }
        let pattern = self.order.iter().map(|&position| &self.pattern[position]);
        Some(
            if crate::gate::Store::eligible(self.pattern.len())
                && let Some(shared) = &self.shared
            {
                crate::gate::Gate::shared(pattern, shared.gate())
            } else {
                crate::gate::Gate::new(pattern)
            },
        )
    }

    pub(crate) fn prepare(
        &self,
        position: usize,
        index: &crate::index::Index,
        site: usize,
    ) -> crate::particle::Match {
        let location = index.location(site);
        if location.world().is_none()
            || self.pattern[position]
                .iter()
                .any(|term| matches!(term.value, crate::program::Symbol::Rule(_)))
        {
            if self.pattern[position].is_empty() {
                return crate::particle::Match::impossible();
            }
            return crate::particle::Match::new(
                &self.pattern[position],
                &index.particle(site, self.pattern[position].iter().cloned()),
            );
        }
        let world = &index.state.world[index.world(site)];
        let particle = &world.particle;
        if self.pattern[position].len() < 8 {
            return crate::particle::Match::new(&self.pattern[position], particle);
        }
        let pattern = self.fragment[position].get_or_init(|| {
            if let Some(shared) = &self.shared {
                return shared.compile(&self.pattern[position]);
            }
            let pattern = self.pattern[position]
                .iter()
                .map(|term| Term::new(term.value, term.capture))
                .collect::<Vec<_>>();
            Arc::new(crate::pattern::Pattern::new(&pattern))
        });
        if particle.len() >= 32
            && let Some(shared) = &self.shared
        {
            return shared.select(pattern, world);
        }
        crate::particle::Match::compiled(pattern, particle)
    }

    pub(crate) fn shared(
        pattern: Arc<Vec<Vec<Term>>>,
        index: &crate::index::Index,
        frame: usize,
        store: &Arc<Store>,
    ) -> Self {
        Self {
            shared: store.subscribe(),
            ..Self::new(pattern, index, frame)
        }
    }

    pub(crate) fn retained(&self) -> usize {
        self.pattern.iter().map(Vec::len).sum::<usize>()
            + self.order.len()
            + self.candidate.iter().map(Vec::len).sum::<usize>()
            + self.candidate.len()
    }
}

#[cfg(test)]
#[path = "test/selection.rs"]
mod test;
