use crate::gate::Gate;
use crate::slot::Slot;
use crate::term::Term;
use std::sync::Arc;
use std::task::Poll;

pub struct Search {
    selection: Arc<crate::selection::Selection>,
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
        Self::prepared(
            Arc::new(crate::selection::Selection::new(
                Arc::new(pattern),
                &index,
                frame,
            )),
            index,
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
            gate: (pattern.len() > 1).then(|| {
                Gate::new(
                    selection
                        .order
                        .iter()
                        .map(|&position| pattern[position].clone())
                        .collect(),
                )
            }),
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

    pub(crate) fn retained(&self) -> usize {
        self.selection.retained() + self.resident()
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
            self.particle = Some(crate::particle::Match::new(
                &self.selection.pattern[self.selection.order[position]],
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
