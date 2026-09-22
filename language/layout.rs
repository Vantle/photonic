use crate::state::State;

pub(crate) struct Layout {
    pub reach: crate::reachability::Index,
    pub cell: usize,
    pub resource: usize,
}

impl Layout {
    pub fn advance(
        &self,
        source: &State,
        state: &State,
        change: &crate::change::Change,
        reach: crate::reachability::Index,
    ) -> Self {
        let removed = change
            .world
            .iter()
            .map(|&world| source.world[world].particle.len())
            .sum::<usize>()
            + change
                .frame
                .iter()
                .filter(|frame| self.reach.frame.binary_search(frame).is_ok())
                .map(|&frame| source.frame[frame].size())
                .sum::<usize>();
        let inserted = state
            .world
            .range(change.insertion.clone())
            .map(|world| world.particle.len())
            .sum::<usize>()
            + change
                .frame
                .iter()
                .filter(|frame| reach.frame.binary_search(frame).is_ok())
                .map(|&frame| state.frame[frame].size())
                .sum::<usize>();
        let newest = state
            .world
            .range(change.insertion.clone())
            .flat_map(|world| &world.particle)
            .chain(
                change
                    .frame
                    .iter()
                    .filter_map(|&frame| state.frame.get(frame))
                    .flat_map(|frame| frame.token()),
            )
            .map(|token| token.id + 1)
            .max()
            .unwrap_or(0);
        let resource = if newest >= self.resource {
            newest
        } else if change
            .world
            .iter()
            .flat_map(|&world| &source.world[world].particle)
            .chain(
                change
                    .frame
                    .iter()
                    .filter_map(|&frame| source.frame.get(frame))
                    .flat_map(|frame| frame.token()),
            )
            .any(|token| token.id + 1 == self.resource)
        {
            state
                .world
                .iter()
                .flat_map(|world| &world.particle)
                .chain(state.frame.iter().flat_map(|frame| frame.token()))
                .map(|token| token.id + 1)
                .max()
                .unwrap_or(0)
        } else {
            self.resource
        };
        Self {
            reach,
            cell: self.cell - removed + inserted,
            resource,
        }
    }

    pub fn new(state: &State) -> Self {
        let reach = crate::reachability::Index::new(state);
        let cell = state
            .world
            .iter()
            .map(|world| world.particle.len())
            .sum::<usize>()
            + reach
                .frame
                .iter()
                .map(|&index| state.frame[index].size())
                .sum::<usize>();
        let resource = state
            .world
            .iter()
            .flat_map(|world| &world.particle)
            .chain(state.frame.iter().flat_map(|frame| frame.token()))
            .map(|token| token.id)
            .max()
            .map_or(0, |id| id + 1);
        Self {
            reach,
            cell,
            resource,
        }
    }
}
