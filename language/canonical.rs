use crate::ordering::Ordering;
use crate::refinement::Refinement;
use crate::state::{Canonical, State};
use std::sync::{Arc, OnceLock};

mod renaming;

pub(crate) mod storage;

pub struct Search {
    state: Arc<State>,
    refinement: OnceLock<Box<Refinement>>,
    world: Ordering,
    frame: Option<Ordering>,
    selected: Vec<usize>,
    best: Option<Canonical>,
    complete: bool,
}

impl Search {
    pub fn new(state: Arc<State>) -> Self {
        let refinement = OnceLock::new();
        let key = |index: usize| {
            let world = &state.world[index];
            let mut particle = world
                .particle
                .iter()
                .map(|token| token.value)
                .collect::<Vec<_>>();
            particle.sort();
            (state.chain(world.frame), particle)
        };
        let world = Ordering::new(0..state.world.len(), key).refine(|index| {
            refinement
                .get_or_init(|| Box::new(Refinement::new(&state)))
                .world[index]
        });
        let world = if state.world.len() >= 4 {
            world.quotient(|| crate::symmetry::world(&state))
        } else {
            world
        };
        Self {
            state,
            refinement,
            world,
            frame: None,
            selected: Vec::new(),
            best: None,
            complete: false,
        }
    }

    pub(crate) fn cost(&self) -> usize {
        self.state.world.len()
    }

    pub fn step(&mut self) -> bool {
        #[cfg(feature = "measurement")]
        let _measurement = crate::measurement::profile::Scope::new(
            crate::measurement::profile::Phase::Canonicalization,
        );
        if self.complete {
            return true;
        }
        if let Some(frame) = self.frame.as_mut().and_then(Iterator::next) {
            let mut order = vec![0];
            order.extend(frame);
            let value = if let Some(refinement) = self.refinement.get() {
                renaming::rename(&self.state, &refinement.incidence, &self.selected, &order)
            } else {
                self.state.rename(&self.selected, &order)
            };
            if self
                .best
                .as_ref()
                .is_none_or(|best| value.state < best.state)
            {
                self.best = Some(value);
            }
            return false;
        }
        let Some(world) = self.world.next() else {
            self.complete = true;
            return true;
        };
        let mut occupied = vec![Vec::new(); self.state.frame.len()];
        let mut capture = vec![Vec::new(); self.state.frame.len()];
        for (position, &source) in world.iter().enumerate() {
            let value = &self.state.world[source];
            occupied[value.frame].push(position);
            for token in &value.particle {
                if let Some(frame) = token.capture {
                    capture[frame].push((position, token.value));
                }
            }
        }
        for capture in &mut capture {
            capture.sort_unstable();
        }
        self.frame = Some(
            Ordering::new(
                self.state
                    .reachable()
                    .into_iter()
                    .filter(|&index| index != 0),
                |index| (self.state.chain(index), &occupied[index], &capture[index]),
            )
            .refine(|index| {
                self.refinement
                    .get_or_init(|| Box::new(Refinement::new(&self.state)))
                    .frame[index]
            }),
        );
        self.selected = world;
        false
    }

    pub fn finish(self) -> Option<Canonical> {
        if self.complete { self.best } else { None }
    }
}

#[cfg(test)]
#[path = "test/canonical.rs"]
mod test;
