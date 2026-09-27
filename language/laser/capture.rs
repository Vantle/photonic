use super::passage::Passage;
use super::pool::Pool;
use crate::basis::Set;
use crate::flow::Flow;
use crate::place::Place;
use crate::state::{Canonical, State};
use std::collections::HashMap;
use std::sync::Arc;

// A rule captured by a frame made after a configuration imports that frame's environment from the
// configuration where the match was found, attached through the flow back to it; carrying the
// attachment with the match lets the import happen wherever the capture stops existing.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct Capture {
    pub origin: usize,
    pub frame: usize,
    pub current: Option<usize>,
    attachment: Vec<(usize, Option<usize>)>,
    resource: Vec<(Place, u32)>,
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub(super) struct Environment {
    state: State,
    frame: Vec<Option<usize>>,
    resource: Vec<(Place, u32)>,
}

fn place(state: &State, index: usize) -> impl Iterator<Item = Place> + '_ {
    let value = &state.frame[index];
    value
        .particle
        .iter()
        .map(move |token| Place::Context(index, token.id))
        .chain(
            value
                .held
                .iter()
                .map(move |token| Place::Held(index, token.id)),
        )
}

fn enclosure(state: &State, frame: usize) -> Vec<usize> {
    let mut selected = vec![false; state.frame.len()];
    selected[0] = true;
    let mut pending = vec![frame];
    while let Some(index) = pending.pop() {
        if std::mem::replace(&mut selected[index], true) {
            continue;
        }
        let value = &state.frame[index];
        pending.extend(value.parent);
        pending.extend(value.lexical);
        pending.extend(
            value
                .particle
                .iter()
                .chain(&value.held)
                .filter_map(|token| token.capture),
        );
    }
    (1..state.frame.len())
        .filter(|&index| selected[index])
        .collect()
}

impl Capture {
    pub fn new(frame: usize, state: &State, origin: usize, pool: &mut Pool) -> Option<Arc<Self>> {
        if frame == 0 {
            return None;
        }
        let enclosed = enclosure(state, frame);
        let resource = enclosed
            .iter()
            .flat_map(|&index| place(state, index))
            .map(|place| (place, pool.basis(Set::single(place))))
            .collect();
        Some(Arc::new(Self {
            origin,
            frame,
            current: Some(frame),
            attachment: enclosed
                .into_iter()
                .map(|index| (index, Some(index)))
                .collect(),
            resource,
        }))
    }

    pub fn carry(&self, event: usize, flow: &Passage, pool: &Pool) -> Arc<Self> {
        Arc::new(Self {
            origin: self.origin,
            frame: self.frame,
            current: self.current.and_then(|frame| flow.frame[frame]),
            attachment: self
                .attachment
                .iter()
                .map(|&(index, value)| (index, value.and_then(|frame| flow.frame[frame])))
                .collect(),
            resource: self
                .resource
                .iter()
                .map(|&(place, basis)| (place, pool.carry(basis, event)))
                .collect(),
        })
    }

    pub fn basis(&self) -> impl Iterator<Item = u32> + '_ {
        self.resource.iter().map(|&(_, basis)| basis)
    }

    pub fn flow(&self, origin: &State, pool: &Pool) -> Flow {
        let mut frame = vec![None; origin.frame.len()];
        frame[0] = Some(0);
        for &(index, value) in &self.attachment {
            frame[index] = value;
        }
        let attached = self
            .resource
            .iter()
            .map(|&(place, basis)| (place, pool.place(basis).clone()))
            .collect::<HashMap<_, _>>();
        let resource = (0..origin.frame.len())
            .flat_map(|index| place(origin, index))
            .map(|place| (place, attached.get(&place).cloned().unwrap_or_default()))
            .collect();
        Flow {
            resource,
            context: vec![Set::default(); origin.world.len()],
            frame,
        }
    }

    pub fn environment(&self, canonical: &Canonical) -> Environment {
        let mut frame = vec![None; canonical.state.frame.len()];
        for &(index, value) in &self.attachment {
            if let Some(position) = canonical.frame[index] {
                frame[position] = value;
            }
        }
        let mut resource = self
            .resource
            .iter()
            .filter_map(|&(place, basis)| Some((canonical.place(place)?, basis)))
            .collect::<Vec<_>>();
        resource.sort_unstable_by_key(|(place, _)| *place);
        Environment {
            state: canonical.state.clone(),
            frame,
            resource,
        }
    }
}
