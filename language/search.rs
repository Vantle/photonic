use crate::matching::{Gate, Slot, Term};
use std::sync::Arc;
use std::task::Poll;

pub struct Search {
    pattern: Vec<Vec<Term>>,
    order: Vec<usize>,
    index: Arc<crate::index::Index>,
    candidate: Vec<Vec<usize>>,
    cursor: Vec<usize>,
    world: usize,
    position: usize,
    particle: Option<crate::particle::Match>,
    gate: Option<Gate>,
    empty: bool,
}

impl Search {
    pub fn new(pattern: Vec<Vec<Term>>, index: Arc<crate::index::Index>, frame: usize) -> Self {
        let mut candidate = pattern
            .iter()
            .map(|pattern| index.candidate(pattern, frame))
            .collect::<Vec<_>>();
        if candidate.iter().any(Vec::is_empty) || !crate::assignment::feasible(&candidate) {
            candidate.clear();
        }
        let mut order = (0..pattern.len()).collect::<Vec<_>>();
        if !candidate.is_empty() {
            order.sort_by_key(|&position| candidate[position].len());
            candidate = order
                .iter()
                .map(|&position| candidate[position].clone())
                .collect();
        }
        let pattern = order
            .iter()
            .map(|&position| pattern[position].clone())
            .collect::<Vec<_>>();
        Self {
            gate: (pattern.len() > 1).then(|| Gate::new(pattern.clone())),
            candidate,
            cursor: vec![0; pattern.len()],
            pattern,
            order,
            index,
            world: 0,
            position: 0,
            particle: None,
            empty: false,
        }
    }

    pub(crate) fn viable(&self) -> bool {
        self.pattern.is_empty() || !self.candidate.is_empty()
    }

    pub(crate) fn retained(&self) -> usize {
        self.candidate.len()
            + self.candidate.iter().map(Vec::len).sum::<usize>()
            + self.cursor.len()
            + self.gate.as_ref().map_or(0, Gate::retained)
            + self
                .particle
                .as_ref()
                .map_or(0, crate::particle::Match::retained)
    }

    pub fn step(&mut self) -> Poll<Option<Vec<Slot>>> {
        if self.pattern.is_empty() {
            if self.empty {
                return Poll::Ready(None);
            }
            self.empty = true;
            return Poll::Ready(Some(Vec::new()));
        }
        if let Some(gate) = self.gate.as_mut()
            && gate.pending()
        {
            return gate.step().map_or(Poll::Pending, |mut value| {
                for slot in &mut value {
                    slot.position = self.order[slot.position];
                }
                value.sort_by_key(|slot| slot.position);
                Poll::Ready(Some(value))
            });
        }
        if self.particle.is_none() {
            let next = self
                .candidate
                .iter()
                .enumerate()
                .filter_map(|(position, candidate)| {
                    candidate
                        .get(self.cursor[position])
                        .map(|&world| (world, position))
                })
                .min();
            let Some((world, position)) = next else {
                return Poll::Ready(None);
            };
            self.world = world;
            self.position = position;
            self.particle = Some(crate::particle::Match::new(
                &self.pattern[position],
                &self.index.state.world[world].particle,
            ));
        }
        let particle = self.particle.as_mut().unwrap();
        match particle.step() {
            Poll::Ready(Some(token)) => {
                let slot = Slot {
                    world: self.world,
                    position: self.position,
                    token,
                };
                let Some(gate) = self.gate.as_mut() else {
                    return Poll::Ready(Some(vec![slot]));
                };
                gate.enqueue(slot);
            }
            Poll::Ready(None) => {
                self.cursor[self.position] += 1;
                self.particle = None;
            }
            Poll::Pending => {}
        }
        Poll::Pending
    }
}
