use crate::profile;
use crate::state::State;

// The coarsest equitable coloring of a configuration's incidence graph, numbered the same way for
// every renaming of the configuration.
pub struct Refinement {
    pub color: Vec<usize>,
    pub incidence: crate::incidence::Incidence,
}

impl Refinement {
    pub fn new(state: &State) -> Self {
        let _scope = profile::Scope::new(profile::Phase::Refinement);
        let incidence = crate::incidence::Incidence::new(state);
        let color = crate::partition::refine(
            &incidence.edge,
            crate::partition::classify(&incidence.label),
        );
        Self { color, incidence }
    }

    #[inline]
    pub fn world(&self, index: usize) -> usize {
        self.color[index]
    }

    #[inline]
    pub fn frame(&self, index: usize) -> usize {
        self.color[self.incidence.frame[index]]
    }
}
