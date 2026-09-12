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
    anchor: Vec<Option<usize>>,
}

impl Search {
    pub fn new(state: Arc<State>) -> Self {
        Self::anchored(state, Vec::new())
    }

    pub fn anchored(state: Arc<State>, anchor: Vec<Option<usize>>) -> Self {
        let refinement = Refinement::anchored(&state, &anchor);
        let world = Ordering::new(0..state.world.len(), |index| {
            let world = &state.world[index];
            let mut particle = world
                .particle
                .iter()
                .map(|token| token.value.rename(&mut |_| 0))
                .collect::<Vec<_>>();
            particle.sort();
            (state.chain(world.frame), particle, refinement.world[index])
        });
        Self {
            state,
            refinement,
            world,
            frame: None,
            selected: Vec::new(),
            best: None,
            complete: false,
            anchor,
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
            if self.best.as_ref().is_none_or(|best| {
                (&value.state, self.mapping(&value))
                    .cmp(&(&best.state, self.mapping(best)))
                    .is_lt()
            }) {
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
                            .filter(move |token| token.value.capture().contains(&index))
                            .map(move |token| (position, token.value.rename(&mut |_| 0)))
                    })
                    .collect::<Vec<_>>();
                capture.sort();
                (
                    self.state.chain(index),
                    occupied,
                    capture,
                    self.refinement.frame[index],
                    self.anchor.get(index).copied().flatten(),
                )
            },
        ));
        self.selected = world;
        false
    }

    fn mapping(&self, value: &Canonical) -> Vec<Option<usize>> {
        let mut result = vec![None; value.state.frame.len()];
        for (index, target) in value.frame.iter().enumerate() {
            if let Some(target) = target {
                result[*target] = self.anchor.get(index).copied().flatten();
            }
        }
        result
    }

    pub fn finish(self) -> Option<Canonical> {
        if self.complete { self.best } else { None }
    }
}
