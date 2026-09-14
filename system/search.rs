use crate::matching::{Gate, Slot, Term};
use crate::program::Symbol;
use crate::state::Token;
use std::sync::Arc;
use std::task::Poll;

struct Particle {
    candidate: Vec<Vec<usize>>,
    selected: Vec<usize>,
    cursor: Vec<usize>,
    complete: bool,
}

impl Particle {
    fn new(pattern: &[Term], particle: &[Token]) -> Self {
        let candidate = pattern
            .iter()
            .map(|term| {
                particle
                    .iter()
                    .filter(|token| {
                        token.value == term.value
                            && (matches!(term.value, Symbol::Atom(_))
                                || token.capture == term.capture)
                    })
                    .map(|token| token.id)
                    .collect()
            })
            .collect::<Vec<Vec<_>>>();
        let complete = candidate.iter().any(Vec::is_empty);
        Self {
            candidate,
            selected: Vec::new(),
            cursor: vec![0; pattern.len()],
            complete,
        }
    }

    fn step(&mut self) -> Poll<Option<Vec<usize>>> {
        if self.complete {
            return Poll::Ready(None);
        }
        let depth = self.selected.len();
        if depth == self.candidate.len() {
            let result = self.selected.clone();
            if self.selected.pop().is_none() {
                self.complete = true;
            }
            return Poll::Ready(Some(result));
        }
        let cursor = self.cursor[depth];
        if cursor == self.candidate[depth].len() {
            self.cursor[depth] = 0;
            if self.selected.pop().is_none() {
                self.complete = true;
            }
            return Poll::Pending;
        }
        self.cursor[depth] += 1;
        let token = self.candidate[depth][cursor];
        let previous = self
            .selected
            .iter()
            .rev()
            .find(|token| self.candidate[depth].contains(token));
        if self.selected.contains(&token) || previous.is_some_and(|&previous| previous >= token) {
            return Poll::Pending;
        }
        self.selected.push(token);
        Poll::Pending
    }
}

pub struct Search {
    pattern: Vec<Vec<Term>>,
    index: Arc<crate::index::Index>,
    candidate: Vec<Vec<usize>>,
    cursor: Vec<usize>,
    world: usize,
    position: usize,
    particle: Option<Particle>,
    gate: Gate,
    empty: bool,
}

impl Search {
    pub fn new(pattern: Vec<Vec<Term>>, index: Arc<crate::index::Index>, frame: usize) -> Self {
        let mut candidate = pattern
            .iter()
            .map(|pattern| index.candidate(pattern, frame))
            .collect::<Vec<_>>();
        if candidate.iter().any(Vec::is_empty) {
            candidate.clear();
        }
        Self {
            gate: Gate::new(pattern.clone()),
            candidate,
            cursor: vec![0; pattern.len()],
            pattern,
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
            + self.gate.retained()
            + self.particle.as_ref().map_or(0, |particle| {
                particle.candidate.iter().map(Vec::len).sum::<usize>()
                    + particle.selected.len()
                    + particle.cursor.len()
            })
    }

    pub fn step(&mut self) -> Poll<Option<Vec<Slot>>> {
        if self.pattern.is_empty() {
            if self.empty {
                return Poll::Ready(None);
            }
            self.empty = true;
            return Poll::Ready(Some(Vec::new()));
        }
        if self.gate.pending() {
            return self
                .gate
                .step()
                .map_or(Poll::Pending, |value| Poll::Ready(Some(value)));
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
            self.particle = Some(Particle::new(
                &self.pattern[position],
                &self.index.state.world[world].particle,
            ));
        }
        let particle = self.particle.as_mut().unwrap();
        match particle.step() {
            Poll::Ready(Some(token)) => self.gate.enqueue(Slot {
                world: self.world,
                position: self.position,
                token,
            }),
            Poll::Ready(None) => {
                self.cursor[self.position] += 1;
                self.particle = None;
            }
            Poll::Pending => {}
        }
        Poll::Pending
    }
}
