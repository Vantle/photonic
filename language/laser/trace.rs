use super::capture::{self, Capture};
use super::pool::{Pool, Reading};
use super::scan::Match;
use crate::application::Owner;
use crate::basis::Set;
use crate::flow::Binding;
use crate::place::Place;
use crate::program::Symbol;
use crate::state::State;
use smallvec::SmallVec;
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Occurrence {
    basis: u64,
    value: Symbol,
    capture: Option<usize>,
}

// A trace waits to be carried across an event until the event's table learns an image it needs.
pub(super) struct Wait;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct Trace {
    pub rule: usize,
    pub frame: usize,
    pub capture: Option<Arc<Capture>>,
    read: u64,
    world: u64,
    occurrence: SmallVec<[Occurrence; 4]>,
}

impl Trace {
    pub fn initial(
        found: &Match,
        state: &State,
        capture: Option<Arc<Capture>>,
        pool: &mut Pool,
    ) -> Option<Self> {
        let mut selected = BTreeSet::new();
        let mut occurrence = SmallVec::new();
        let mut world = BTreeSet::new();
        for slot in &found.selection {
            if let Some(index) = slot.location.world() {
                world.insert(index);
            }
            for &id in &slot.token {
                let place = state.resolve(slot.location, id)?;
                if !selected.insert(place) {
                    return None;
                }
                let token = state.token(place)?;
                occurrence.push(Occurrence {
                    basis: pool.basis(Set::single(place)),
                    value: token.value,
                    capture: token.capture,
                });
            }
        }
        occurrence.sort_unstable();
        Some(Self {
            rule: found.rule,
            frame: found.frame,
            capture,
            read: pool.basis(Set::single(found.read)),
            world: pool.world(world.into_iter().collect()),
            occurrence,
        })
    }

    // The trace carried back across an event, none when its frame did not exist before the event; a
    // trace waits while the event's table lacks an image it takes.
    pub fn carry(
        &self,
        reading: &mut Reading<'_>,
        store: &capture::Store,
    ) -> Result<Option<Self>, Wait> {
        let Some(frame) = reading.frame(self.frame) else {
            return Ok(None);
        };
        let mut occurrence = SmallVec::<[Occurrence; 4]>::with_capacity(self.occurrence.len());
        for value in &self.occurrence {
            occurrence.push(Occurrence {
                basis: reading.basis(value.basis),
                value: value.value,
                capture: value.capture.and_then(|capture| reading.frame(capture)),
            });
        }
        let capture = self
            .capture
            .as_ref()
            .and_then(|capture| store.carry(capture, reading));
        let read = reading.basis(self.read);
        let world = reading.world(self.world);
        if reading.lacking() > 0 {
            return Err(Wait);
        }
        occurrence.sort_unstable();
        Ok(Some(Self {
            rule: self.rule,
            frame,
            capture,
            read,
            world,
            occurrence,
        }))
    }

    pub fn share(mut self, store: &capture::Store) -> Self {
        self.capture = self.capture.map(|capture| store.share(capture));
        self
    }

    // Where the trace's rule lives: a frame of its configuration, or a capture no longer there,
    // whose environment the rule applies through.
    pub fn owner(&self) -> Owner<&Capture> {
        let Some(capture) = &self.capture else {
            return Owner::Frame(0);
        };
        match capture.current {
            Some(frame) => Owner::Frame(frame),
            None => Owner::Capture(capture.as_ref()),
        }
    }

    pub fn binding(&self, state: &State, pool: &Pool) -> Option<Binding> {
        let mut footprint = SmallVec::<[Place; 8]>::new();
        let mut exact = SmallVec::<[Place; 8]>::new();
        for value in &self.occurrence {
            let basis = pool.place(value.basis);
            footprint.extend(basis.iter());
            if basis.len() != 1 {
                continue;
            }
            let place = basis.iter().next()?;
            if state
                .token(place)
                .is_some_and(|token| token.value == value.value && token.capture == value.capture)
            {
                exact.push(place);
            }
        }
        footprint.sort_unstable();
        footprint.dedup();
        exact.sort_unstable();
        exact.dedup();
        let mut world = pool
            .site(self.world)
            .iter()
            .collect::<SmallVec<[usize; 8]>>();
        for place in &footprint {
            match place {
                Place::World(index, _) => world.push(*index),
                Place::Context(_, _) => {}
                Place::Held(_, _) => return None,
            }
        }
        world.sort_unstable();
        world.dedup();
        if world
            .iter()
            .any(|&index| state.world[index].frame != self.frame)
        {
            return None;
        }
        Some(Binding {
            world: world.into_iter().collect(),
            footprint: footprint.into_iter().collect(),
            exact: exact.into_iter().collect(),
            read: pool.place(self.read).iter().collect(),
        })
    }
}
