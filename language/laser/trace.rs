use super::capture::Capture;
use super::passage::Passage;
use super::pool::{Demand, Pool};
use super::scan::Match;
use crate::basis::Set;
use crate::flow::Binding;
use crate::place::Place;
use crate::program::Symbol;
use crate::state::State;
use smallvec::SmallVec;
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct Occurrence {
    basis: u32,
    value: Symbol,
    capture: Option<usize>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct Trace {
    pub rule: usize,
    pub frame: usize,
    pub capture: Option<Arc<Capture>>,
    read: u32,
    world: u32,
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
            read: pool.basis(found.read.map_or_else(Set::default, Set::single)),
            world: pool.world(world.into_iter().collect()),
            occurrence,
        })
    }

    pub fn need(&self, event: usize, flow: &Passage, pool: &Pool, demand: &mut Demand) {
        if flow.frame(self.frame).is_none() {
            return;
        }
        let capture = self.capture.iter().flat_map(|capture| capture.basis());
        let basis = self
            .occurrence
            .iter()
            .map(|value| value.basis)
            .chain([self.read])
            .chain(capture);
        pool.demand(event, basis, self.world, demand);
    }

    pub fn carry(&self, event: usize, flow: &Passage, pool: &Pool) -> Option<Self> {
        let frame = flow.frame(self.frame)?;
        let mut occurrence = self
            .occurrence
            .iter()
            .map(|value| Occurrence {
                basis: pool.carry(value.basis, event),
                value: value.value,
                capture: value.capture.and_then(|capture| flow.frame(capture)),
            })
            .collect::<SmallVec<[Occurrence; 4]>>();
        occurrence.sort_unstable();
        Some(Self {
            rule: self.rule,
            frame,
            capture: self
                .capture
                .as_ref()
                .map(|capture| capture.carry(event, flow, pool)),
            read: pool.carry(self.read, event),
            world: pool.follow(self.world, event),
            occurrence,
        })
    }

    pub fn current(&self) -> Option<usize> {
        self.capture
            .as_ref()
            .map_or(Some(0), |capture| capture.current)
    }

    pub fn binding(&self, state: &State, pool: &Pool) -> Option<Binding> {
        let mut footprint = SmallVec::<[Place; 8]>::new();
        let mut exact = SmallVec::<[Place; 8]>::new();
        for value in &self.occurrence {
            let basis = pool.place(value.basis);
            footprint.extend(basis.iter().copied());
            if basis.len() != 1 {
                continue;
            }
            let place = *basis.first()?;
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
            .copied()
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
            read: pool.place(self.read).clone(),
        })
    }
}
