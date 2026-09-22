use crate::state::State;

pub struct Refinement {
    pub world: Vec<usize>,
    pub frame: Vec<usize>,
    pub incidence: crate::incidence::Incidence,
}

impl Refinement {
    pub fn new(state: &State) -> Self {
        #[cfg(feature = "measurement")]
        let _measurement =
            crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Refinement);
        let incidence = crate::incidence::Incidence::new(state);
        let color = crate::partition::refine(
            &incidence.edge,
            crate::partition::classify(&incidence.label),
        );
        Self {
            world: color[..state.world.len()].to_vec(),
            frame: incidence.frame.iter().map(|&index| color[index]).collect(),
            incidence,
        }
    }
}
