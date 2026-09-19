use crate::state::State;

pub(crate) struct Layout {
    pub reachable: Vec<usize>,
    pub cell: usize,
    pub resource: usize,
}

impl Layout {
    pub fn new(state: &State, reachable: Vec<usize>) -> Self {
        let cell = state
            .world
            .iter()
            .map(|world| world.particle.len())
            .sum::<usize>()
            + reachable
                .iter()
                .map(|&index| state.frame[index].held.len())
                .sum::<usize>();
        let resource = state
            .world
            .iter()
            .flat_map(|world| &world.particle)
            .chain(state.frame.iter().flat_map(|frame| &frame.held))
            .map(|token| token.id)
            .max()
            .map_or(0, |id| id + 1);
        Self {
            reachable,
            cell,
            resource,
        }
    }
}
