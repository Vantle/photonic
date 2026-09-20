use crate::change::Change;
use crate::sequence::List;
use crate::state::State;
use smallvec::SmallVec;
use std::sync::Arc;

pub(crate) struct Index {
    anchor: List<usize>,
    pub frame: Arc<Vec<usize>>,
}

fn reference(world: &crate::state::World) -> impl Iterator<Item = usize> + '_ {
    std::iter::once(world.frame).chain(world.particle.iter().filter_map(|token| token.capture))
}

impl Index {
    pub fn new(state: &State) -> Self {
        let mut anchor = std::iter::repeat_n(0, state.frame.len()).collect::<List<_>>();
        anchor[0] = 1;
        for frame in state.world.iter().flat_map(|world| reference(world)) {
            anchor[frame] += 1;
        }
        Self {
            anchor,
            frame: Arc::new(state.reachable()),
        }
    }

    pub fn advance(&self, source: &State, state: &State, change: &Change) -> Self {
        let mut anchor = self.anchor.clone();
        while anchor.len() < state.frame.len() {
            anchor.push(0);
        }
        let mut affected = SmallVec::<[usize; 8]>::new();
        for frame in change
            .world
            .iter()
            .flat_map(|&world| reference(&source.world[world]))
        {
            anchor[frame] -= 1;
            affected.push(frame);
        }
        for frame in state
            .world
            .range(change.insertion.clone())
            .flat_map(|world| reference(world))
        {
            anchor[frame] += 1;
            affected.push(frame);
        }
        let unchanged = change.frame.is_empty()
            && affected.iter().all(|&frame| {
                (anchor[frame] == 0) == (self.anchor.get(frame).copied().unwrap_or(0) == 0)
            });
        let frame = if unchanged {
            self.frame.clone()
        } else {
            Arc::new(state.reachable())
        };
        anchor.truncate(frame.last().unwrap() + 1);
        Self { anchor, frame }
    }

    pub fn retained(&self) -> usize {
        self.anchor.len() + self.frame.len()
    }
}
