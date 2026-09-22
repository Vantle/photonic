use crate::change::Change;
use crate::sequence::List;
use crate::state::State;
use smallvec::SmallVec;
use std::sync::Arc;

mod network;

fn admitted(state: &State) -> bool {
    state.frame.len() >= 64 && state.world.len() / state.frame.len() >= 4
}

enum Storage {
    Retained(network::Network),
    Evicted,
}

#[derive(Clone)]
pub(crate) struct Index {
    storage: Option<Arc<Storage>>,
    anchor: List<usize>,
    pub frame: Arc<Vec<usize>>,
}

fn closure(state: &State, anchor: &List<usize>) -> Vec<usize> {
    state.closure(
        anchor
            .iter()
            .enumerate()
            .filter_map(|(frame, &count)| (count > 0).then_some(frame)),
    )
}

impl Index {
    pub fn new(state: &State) -> Self {
        let mut anchor = std::iter::repeat_n(0, state.frame.len()).collect::<List<_>>();
        anchor[0] = 1;
        for frame in state.world.iter().flat_map(|world| world.reference()) {
            anchor[frame] += 1;
        }
        let frame = Arc::new(closure(state, &anchor));
        Self {
            storage: admitted(state)
                .then(|| Arc::new(Storage::Retained(network::Network::new(state, &frame)))),
            anchor,
            frame,
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
            .flat_map(|&world| source.world[world].reference())
        {
            anchor[frame] -= 1;
            affected.push(frame);
        }
        for frame in state
            .world
            .range(change.insertion.clone())
            .flat_map(|world| world.reference())
        {
            anchor[frame] += 1;
            affected.push(frame);
        }
        let unchanged = change.frame.is_empty()
            && affected.iter().all(|&frame| {
                (anchor[frame] == 0) == (self.anchor.get(frame).copied().unwrap_or(0) == 0)
            });
        let enabled = !matches!(self.storage.as_deref(), Some(Storage::Evicted));
        let retained = self
            .network()
            .filter(|_| state.frame.len() >= 32 && state.world.len() / state.frame.len() >= 2);
        let (storage, frame) = if unchanged {
            (
                if retained.is_some() || !enabled {
                    self.storage.clone()
                } else {
                    None
                },
                self.frame.clone(),
            )
        } else if let Some(network) = retained {
            let (network, frame) = network.advance(network::Request {
                source,
                state,
                anchor: &anchor,
                affected: &affected,
                changed: &change.frame,
            });
            (Some(Arc::new(Storage::Retained(network))), frame)
        } else {
            let frame = Arc::new(closure(state, &anchor));
            let storage = if !enabled {
                self.storage.clone()
            } else {
                admitted(state)
                    .then(|| Arc::new(Storage::Retained(network::Network::new(state, &frame))))
            };
            (storage, frame)
        };
        anchor.truncate(frame.last().unwrap() + 1);
        Self {
            storage,
            anchor,
            frame,
        }
    }

    fn network(&self) -> Option<&network::Network> {
        match self.storage.as_deref() {
            Some(Storage::Retained(network)) => Some(network),
            _ => None,
        }
    }

    pub fn evict(&mut self) -> usize {
        if matches!(self.storage.as_deref(), Some(Storage::Evicted)) {
            return 0;
        }
        let released = self.network().map_or(0, network::Network::retained);
        self.storage = Some(Arc::new(Storage::Evicted));
        released
    }

    pub fn retained(&self) -> usize {
        self.anchor.len() + self.frame.len() + self.network().map_or(0, network::Network::retained)
    }
}
