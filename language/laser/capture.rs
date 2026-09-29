use super::memo::Memo;
use super::pool::{Pool, Reading};
use super::shard;
use crate::basis::Set;
use crate::executor::Executor;
use crate::flow::Flow;
use crate::place::Place;
use crate::state::{Canonical, State};
use hashing::Builder;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

// A rule captured by a frame made after a configuration imports that frame's environment from the
// configuration where the match was found, attached through the flow back to it; carrying the
// attachment with the match lets the import happen wherever the capture stops existing.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct Capture {
    pub origin: usize,
    pub frame: usize,
    pub current: Option<usize>,
    attachment: Vec<(usize, Option<usize>)>,
    resource: Vec<(Place, u64)>,
}

// Traces carried back along different walks often hold equal captures, so every stored trace
// holds the store's copy and equal captures are kept once. The traces that share a capture carry
// it across the same events, so each carry of a capture across an event is remembered until the
// batch ends; a capture a trace holds lives as long as the trace, so its address names it while it
// is remembered. Carries run in parallel, and shards keep them from waiting on one another.
pub(super) struct Store {
    capture: Vec<Mutex<HashSet<Arc<Capture>, Builder>>>,
    carried: Memo<Carry, Arc<Capture>>,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct Carry {
    event: usize,
    capture: usize,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            capture: shard::empty(),
            carried: Memo::default(),
        }
    }
}

impl Store {
    pub fn share(&self, capture: Arc<Capture>) -> Arc<Capture> {
        let mut set = self.capture[shard::slot(hashing::value(&*capture))]
            .lock()
            .expect("an unpoisoned store");
        if let Some(known) = set.get(&*capture) {
            return known.clone();
        }
        set.insert(capture.clone());
        capture
    }

    // A capture carried across an event; only a carry that lacked no image is remembered.
    pub fn carry(&self, capture: &Arc<Capture>, reading: &mut Reading<'_>) -> Arc<Capture> {
        let key = Carry {
            event: reading.event(),
            capture: Arc::as_ptr(capture) as usize,
        };
        if let Some(found) = self.carried.find(&key) {
            return found;
        }
        let lacking = reading.lacking();
        let carried = Arc::new(capture.carry(reading));
        if reading.lacking() > lacking {
            return carried;
        }
        self.carried.keep(key, carried)
    }

    pub fn forget(&mut self, executor: Option<&Executor>) {
        self.carried.forget(executor);
    }
}

#[derive(Debug, Eq, Hash, PartialEq)]
pub(super) struct Environment {
    state: State,
    frame: Vec<Option<usize>>,
    resource: Vec<(Place, u64)>,
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
        pending.extend(state.frame[index].reference());
    }
    (1..state.frame.len())
        .filter(|&index| selected[index])
        .collect()
}

impl Capture {
    pub fn new(frame: usize, state: &State, origin: usize, pool: &mut Pool) -> Option<Self> {
        if frame == 0 {
            return None;
        }
        let enclosed = enclosure(state, frame);
        let resource = enclosed
            .iter()
            .flat_map(|&index| place(state, index))
            .map(|place| (place, pool.basis(Set::single(place))))
            .collect();
        Some(Self {
            origin,
            frame,
            current: Some(frame),
            attachment: enclosed
                .into_iter()
                .map(|index| (index, Some(index)))
                .collect(),
            resource,
        })
    }

    fn carry(&self, reading: &mut Reading<'_>) -> Self {
        Self {
            origin: self.origin,
            frame: self.frame,
            current: self.current.and_then(|frame| reading.frame(frame)),
            attachment: self
                .attachment
                .iter()
                .map(|&(index, value)| (index, value.and_then(|frame| reading.frame(frame))))
                .collect(),
            resource: self
                .resource
                .iter()
                .map(|&(place, basis)| (place, reading.basis(basis)))
                .collect(),
        }
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
            .map(|&(place, basis)| (place, pool.place(basis).iter().collect::<Set<_>>()))
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
            if let Some(position) = canonical.renaming.frame[index] {
                frame[position] = value;
            }
        }
        let mut resource = self
            .resource
            .iter()
            .filter_map(|&(place, basis)| Some((canonical.renaming.place(place)?, basis)))
            .collect::<Vec<_>>();
        resource.sort_unstable_by_key(|(place, _)| *place);
        Environment {
            state: canonical.state.clone(),
            frame,
            resource,
        }
    }
}
