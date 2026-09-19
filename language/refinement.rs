use crate::state::State;

pub struct Refinement {
    pub world: Vec<usize>,
    pub frame: Vec<usize>,
}

impl Refinement {
    pub fn new(state: &State) -> Self {
        let incidence = crate::incidence::Incidence::new(state);
        let color = crate::partition::refine(
            &incidence.edge,
            crate::partition::classify(&incidence.label),
        );
        Self {
            world: color[..state.world.len()].to_vec(),
            frame: incidence
                .frame
                .into_iter()
                .map(|index| color[index])
                .collect(),
        }
    }
}
