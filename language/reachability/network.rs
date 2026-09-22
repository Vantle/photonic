use crate::basis::Set;
use crate::sequence::List;
use crate::state::State;
use std::collections::BTreeSet;
use std::sync::Arc;

pub(super) struct Request<'state> {
    pub source: &'state State,
    pub state: &'state State,
    pub anchor: &'state List<usize>,
    pub affected: &'state [usize],
    pub changed: &'state [usize],
}

#[derive(Clone)]
pub(super) struct Network {
    incoming: List<Set<usize>>,
    selected: List<bool>,
    edge: usize,
}

fn adjacent(state: &State, source: usize) -> impl Iterator<Item = usize> + '_ {
    state
        .frame
        .get(source)
        .into_iter()
        .flat_map(|frame| frame.reference())
}

impl Network {
    pub fn new(state: &State, frame: &[usize]) -> Self {
        let length = frame.last().unwrap() + 1;
        let mut network = Self {
            incoming: std::iter::repeat_n(Set::default(), length).collect(),
            selected: std::iter::repeat_n(false, length).collect(),
            edge: 0,
        };
        for &source in frame {
            network.selected[source] = true;
            for target in adjacent(state, source) {
                network.insert(source, target);
            }
        }
        network
    }

    fn contains(&self, source: usize) -> bool {
        self.selected.get(source).copied().unwrap_or(false)
    }

    fn insert(&mut self, source: usize, target: usize) {
        let incoming = &self.incoming[target];
        if incoming.contains(&source) {
            return;
        }
        self.incoming[target] = incoming.iter().copied().chain([source]).collect();
        self.edge += 1;
    }

    fn remove(&mut self, source: usize, target: usize) {
        let Some(incoming) = self.incoming.get(target) else {
            return;
        };
        if !incoming.contains(&source) {
            return;
        }
        self.incoming[target] = incoming
            .iter()
            .copied()
            .filter(|&value| value != source)
            .collect();
        self.edge -= 1;
    }

    fn invalidate(
        &mut self,
        source: &State,
        anchor: &List<usize>,
        pending: &mut Vec<usize>,
    ) -> BTreeSet<usize> {
        let mut region = BTreeSet::new();
        while let Some(frame) = pending.pop() {
            if !self.contains(frame) || anchor.get(frame).copied().unwrap_or(0) > 0 {
                continue;
            }
            self.selected[frame] = false;
            region.insert(frame);
            pending.extend(adjacent(source, frame));
        }
        region
    }

    fn restore(&mut self, state: &State, pending: &mut Vec<usize>) -> Vec<usize> {
        let mut added = Vec::new();
        while let Some(frame) = pending.pop() {
            if self.contains(frame) {
                continue;
            }
            self.selected[frame] = true;
            added.push(frame);
            pending.extend(adjacent(state, frame));
        }
        added
    }

    pub fn advance(&self, request: Request<'_>) -> (Self, Arc<Vec<usize>>) {
        let Request {
            source,
            state,
            anchor,
            affected,
            changed,
        } = request;
        let mut network = self.clone();
        while network.selected.len() < state.frame.len() {
            network.selected.push(false);
            network.incoming.push(Set::default());
        }
        let mut pending = affected
            .iter()
            .copied()
            .filter(|&frame| self.contains(frame) && anchor[frame] == 0)
            .collect::<Vec<_>>();
        for &frame in changed {
            if !self.contains(frame) {
                continue;
            }
            pending.extend(adjacent(source, frame));
            for target in adjacent(source, frame) {
                network.remove(frame, target);
            }
            for target in adjacent(state, frame) {
                network.insert(frame, target);
            }
        }
        let region = network.invalidate(source, anchor, &mut pending);
        pending.extend(affected.iter().copied().filter(|&frame| anchor[frame] > 0));
        for &frame in &region {
            if network.incoming[frame]
                .iter()
                .any(|&parent| network.contains(parent))
            {
                pending.push(frame);
            }
        }
        for &frame in changed {
            if network.contains(frame) {
                pending.extend(adjacent(state, frame));
            }
        }
        let added = network.restore(state, &mut pending);
        for &frame in &region {
            if !network.contains(frame) {
                for target in adjacent(state, frame) {
                    network.remove(frame, target);
                }
            }
        }
        for &frame in &added {
            if !self.contains(frame) {
                for target in adjacent(state, frame) {
                    network.insert(frame, target);
                }
            }
        }
        let frame = network
            .selected
            .iter()
            .enumerate()
            .filter_map(|(frame, &selected)| selected.then_some(frame))
            .collect::<Vec<_>>();
        let length = frame.last().unwrap() + 1;
        network.incoming.truncate(length);
        network.selected.truncate(length);
        (network, Arc::new(frame))
    }

    pub fn retained(&self) -> usize {
        self.incoming.len() + self.selected.len() + self.edge
    }
}
