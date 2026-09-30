use super::renaming;
use crate::ordering::Ordering;
use crate::refinement::Refinement;
use crate::state::{Canonical, State};
use smallvec::SmallVec;
use std::sync::{Arc, OnceLock};

// The search over every ordering of a configuration's coherences and frames that its keys and
// refinement leave open, keeping the least renamed configuration.
pub(super) struct Whole {
    pub state: Arc<State>,
    refinement: OnceLock<Box<Refinement>>,
    world: Ordering,
    frame: Option<Ordering>,
    selected: Vec<usize>,
    best: Option<Canonical>,
    complete: bool,
}

impl Whole {
    pub fn new(state: Arc<State>) -> Self {
        let refinement = OnceLock::new();
        let chain = (0..state.frame.len())
            .map(|_| OnceLock::new())
            .collect::<Vec<OnceLock<_>>>();
        let key = |index: usize| {
            let world = &state.world[index];
            let mut particle = world
                .particle
                .iter()
                .map(|token| token.value)
                .collect::<SmallVec<[_; 4]>>();
            particle.sort();
            (
                chain[world.frame].get_or_init(|| state.chain(world.frame)),
                particle,
            )
        };
        let world = Ordering::new(0..state.world.len(), key).refine(|index| {
            refinement
                .get_or_init(|| Box::new(Refinement::new(&state)))
                .world(index)
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

    #[cfg(test)]
    pub fn refined(&self) -> bool {
        self.refinement.get().is_some()
    }

    // Whether no step has been taken yet.
    pub fn fresh(&self) -> bool {
        self.frame.is_none()
    }

    // The refinement, made now if the keys alone told every coherence and frame apart.
    pub fn refinement(&self) -> &Refinement {
        self.refinement
            .get_or_init(|| Box::new(Refinement::new(&self.state)))
    }

    // The steps the whole search takes, once its first step has ordered the frames: one for each
    // ordering of the coherences, one for each ordering of the frames under it, and one to end.
    pub fn cost(&self) -> Option<usize> {
        let frame = self.frame.as_ref()?.total();
        let product = frame
            .and_then(|frame| frame.checked_add(1))
            .zip(self.world.total())
            .and_then(|(frame, world)| frame.checked_mul(world));
        Some(product.map_or(usize::MAX, |product| product.saturating_add(1)))
    }

    pub fn step(&mut self) -> bool {
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
                    .frame(index)
            }),
        );
        self.selected = world;
        false
    }

    pub fn finish(self) -> Option<Canonical> {
        if self.complete { self.best } else { None }
    }
}
