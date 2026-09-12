use crate::ordering::Ordering;
use crate::refinement::Refinement;
use crate::state::{Canonical, State};
use std::sync::Arc;

pub struct Search {
    state: Arc<State>,
    refinement: Refinement,
    world: Ordering,
    frame: Option<Ordering>,
    selected: Vec<usize>,
    best: Option<Canonical>,
    complete: bool,
}

impl Search {
    pub fn new(state: Arc<State>) -> Self {
        let refinement = Refinement::new(&state);
        let mut color = std::collections::BTreeSet::new();
        let representative = (state.world.len() >= 4
            && refinement.world.iter().any(|value| !color.insert(*value)))
        .then(|| crate::symmetry::world(&state));
        let key = |index: usize| {
            let world = &state.world[index];
            let mut particle = world
                .particle
                .iter()
                .map(|token| token.value)
                .collect::<Vec<_>>();
            particle.sort();
            (state.chain(world.frame), particle, refinement.world[index])
        };
        let world = match representative {
            Some(representative)
                if representative
                    .iter()
                    .enumerate()
                    .any(|(index, value)| index != *value) =>
            {
                Ordering::quotient(0..state.world.len(), key, |index| representative[index])
            }
            _ => Ordering::new(0..state.world.len(), key),
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

    pub fn step(&mut self) -> bool {
        if self.complete {
            return true;
        }
        if let Some(frame) = self.frame.as_mut().and_then(Iterator::next) {
            let mut order = vec![0];
            order.extend(frame);
            let value = self.state.rename(&self.selected, &order);
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
        self.frame = Some(Ordering::new(
            self.state
                .reachable()
                .into_iter()
                .filter(|&index| index != 0),
            |index| {
                let occupied = world
                    .iter()
                    .enumerate()
                    .filter_map(|(position, &source)| {
                        (self.state.world[source].frame == index).then_some(position)
                    })
                    .collect::<Vec<_>>();
                let mut capture = world
                    .iter()
                    .enumerate()
                    .flat_map(|(position, &source)| {
                        self.state.world[source]
                            .particle
                            .iter()
                            .filter(move |token| token.capture == Some(index))
                            .map(move |token| (position, token.value))
                    })
                    .collect::<Vec<_>>();
                capture.sort();
                (
                    self.state.chain(index),
                    occupied,
                    capture,
                    self.refinement.frame[index],
                )
            },
        ));
        self.selected = world;
        false
    }

    pub fn finish(self) -> Option<Canonical> {
        if self.complete { self.best } else { None }
    }
}
