use crate::gate::Gate;
use crate::slot::Slot;
use std::sync::Arc;
use std::task::Poll;

pub(super) struct Cursor {
    pub(super) selection: Arc<crate::selection::Selection>,
    pub(super) index: Arc<crate::index::Index>,
    pub(super) candidate: Vec<Vec<usize>>,
    cursor: Vec<usize>,
    world: usize,
    position: usize,
    particle: Option<crate::particle::Match>,
    gate: Option<Gate>,
    empty: bool,
}

impl Cursor {
    pub fn cost(&self) -> usize {
        if self.particle.is_some() || self.gate.as_ref().is_some_and(Gate::pending) {
            return 0;
        }
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
            return 0;
        };
        self.selection.cost(
            self.selection.order[position],
            &self.index.state.world[world],
        )
    }

    pub(crate) fn prepared(
        selection: Arc<crate::selection::Selection>,
        index: Arc<crate::index::Index>,
    ) -> Self {
        let candidate = selection
            .order
            .iter()
            .map(|&position| {
                let candidate = &selection.candidate[position];
                let mut candidate = candidate
                    .iter()
                    .map(|&site| index.world(site))
                    .collect::<Vec<_>>();
                candidate.sort_unstable();
                candidate
            })
            .collect();
        let pattern = &selection.pattern;
        Self {
            gate: selection.gate(),
            candidate,
            cursor: vec![0; pattern.len()],
            selection,
            index,
            world: 0,
            position: 0,
            particle: None,
            empty: false,
        }
    }

    pub(crate) fn viable(&self) -> bool {
        self.selection.viable
    }

    pub(crate) fn resident(&self) -> usize {
        self.candidate.len()
            + self.candidate.iter().map(Vec::len).sum::<usize>()
            + self.cursor.len()
            + self.gate.as_ref().map_or(0, Gate::retained)
            + self
                .particle
                .as_ref()
                .map_or(0, crate::particle::Match::retained)
    }

    pub(crate) fn evict(&mut self) {
        if let Some(gate) = &mut self.gate {
            gate.evict();
        }
    }

    pub fn step(&mut self) -> Poll<Option<Vec<Slot>>> {
        if self.selection.pattern.is_empty() {
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
                    slot.position = self.selection.order[slot.position];
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
            self.particle = Some(self.selection.prepare(
                self.selection.order[position],
                &self.index.state.world[world],
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
