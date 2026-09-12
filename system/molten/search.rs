use crate::matching::{Gate, Slot, Term};
use crate::state::{State, Token};
use std::sync::Arc;
use std::task::Poll;

#[derive(Clone)]
struct Candidate {
    token: usize,
    binding: std::collections::BTreeMap<String, crate::program::Symbol>,
}

struct Particle {
    candidate: Vec<Vec<Candidate>>,
    pattern: Vec<Term>,
    particle: Vec<Token>,
    position: usize,
    token: usize,
    search: Option<crate::value::Search>,
    selected: Vec<usize>,
    cursor: Vec<usize>,
    complete: bool,
}

impl Particle {
    fn new(pattern: &[Term], particle: &[Token], _state: &State) -> Self {
        Self {
            candidate: (0..pattern.len()).map(|_| Vec::new()).collect(),
            pattern: pattern.to_vec(),
            particle: particle.to_vec(),
            position: 0,
            token: 0,
            search: None,
            selected: Vec::new(),
            cursor: vec![0; pattern.len()],
            complete: false,
        }
    }

    fn step(
        &mut self,
    ) -> Poll<
        Option<(
            Vec<usize>,
            std::collections::BTreeMap<String, crate::program::Symbol>,
        )>,
    > {
        if self.position < self.pattern.len() {
            if self.token == 0 {
                if let Some(previous) = (0..self.position)
                    .find(|&index| self.pattern[index] == self.pattern[self.position])
                {
                    self.candidate[self.position] = self.candidate[previous].clone();
                    self.position += 1;
                    return Poll::Pending;
                }
            }
            if self.token == self.particle.len() {
                self.position += 1;
                self.token = 0;
                return Poll::Pending;
            }
            let search = self.search.get_or_insert_with(|| {
                crate::value::Search::new(
                    self.pattern[self.position].value.clone(),
                    self.particle[self.token].value.clone(),
                )
            });
            match search.step() {
                Poll::Pending => {}
                Poll::Ready(None) => {
                    self.search = None;
                    self.token += 1;
                }
                Poll::Ready(Some(binding)) => self.candidate[self.position].push(Candidate {
                    token: self.particle[self.token].id,
                    binding,
                }),
            }
            return Poll::Pending;
        }
        if self.complete {
            return Poll::Ready(None);
        }
        let depth = self.selected.len();
        if depth == self.candidate.len() {
            let mut token = Vec::new();
            let mut binding = std::collections::BTreeMap::new();
            for (position, &index) in self.selected.iter().enumerate() {
                let candidate = &self.candidate[position][index];
                token.push(candidate.token);
                binding.extend(candidate.binding.clone());
            }
            if self.selected.pop().is_none() {
                self.complete = true;
            }
            return Poll::Ready(Some((token, binding)));
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
        let candidate = &self.candidate[depth][cursor];
        for (position, &selected) in self.selected.iter().enumerate() {
            let previous = &self.candidate[position][selected];
            if previous.token == candidate.token
                || self.pattern[position] == self.pattern[depth]
                    && previous.token >= candidate.token
                || previous.binding.iter().any(|(name, value)| {
                    candidate
                        .binding
                        .get(name)
                        .is_some_and(|other| other != value)
                })
            {
                return Poll::Pending;
            }
        }
        self.selected.push(cursor);
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
    constraint: Option<crate::constraint::Constraint>,
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
            constraint: None,
        }
    }

    pub fn constrain(mut self, constraint: Option<crate::constraint::Constraint>) -> Self {
        self.constraint = constraint;
        self
    }

    pub(crate) fn retained(&self) -> usize {
        self.gate.retained()
            + self.particle.as_ref().map_or(0, |particle| {
                particle.candidate.iter().map(Vec::len).sum::<usize>()
                    + particle.selected.len()
                    + particle.cursor.len()
                    + particle
                        .search
                        .as_ref()
                        .map_or(0, crate::value::Search::retained)
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
            let Some(value) = self.gate.step() else {
                return Poll::Pending;
            };
            if self.constraint.as_ref().is_some_and(|constraint| {
                let binding = value.iter().flat_map(|slot| slot.binding.clone()).collect();
                !constraint.accepts(&self.state, &binding)
            }) {
                return Poll::Pending;
            }
            return Poll::Ready(Some(value));
        }
        let Some(world) = self.state.world.get(self.world) else {
            return Poll::Ready(None);
        };
        if world.frame != self.frame || self.position == self.pattern.len() {
            self.world += 1;
            self.position = 0;
            return Poll::Pending;
        }
        let particle = self.particle.get_or_insert_with(|| {
            Particle::new(&self.pattern[self.position], &world.particle, &self.state)
        });
        match particle.step() {
            Poll::Ready(Some((token, binding))) => self.gate.enqueue(Slot {
                binding,
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
