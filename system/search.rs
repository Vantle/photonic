use crate::matching::{Gate, Slot, Term};
use crate::program::Symbol;
use crate::state::{State, Token};
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
    state: Arc<State>,
    frame: usize,
    world: usize,
    position: usize,
    particle: Option<Particle>,
    gate: Gate,
    empty: bool,
}

impl Search {
    pub fn new(pattern: Vec<Vec<Term>>, state: Arc<State>, frame: usize) -> Self {
        Self {
            gate: Gate::new(pattern.clone()),
            pattern,
            state,
            frame,
            world: 0,
            position: 0,
            particle: None,
            empty: false,
        }
    }

    pub(crate) fn retained(&self) -> usize {
        self.gate.retained()
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
        let Some(world) = self.state.world.get(self.world) else {
            return Poll::Ready(None);
        };
        if world.frame != self.frame || self.position == self.pattern.len() {
            self.world += 1;
            self.position = 0;
            return Poll::Pending;
        }
        let particle = self
            .particle
            .get_or_insert_with(|| Particle::new(&self.pattern[self.position], &world.particle));
        match particle.step() {
            Poll::Ready(Some(token)) => self.gate.enqueue(Slot {
                world: self.world,
                position: self.position,
                token,
            }),
            Poll::Ready(None) => {
                self.position += 1;
                self.particle = None;
            }
            Poll::Pending => {}
        }
        Poll::Pending
    }
}
