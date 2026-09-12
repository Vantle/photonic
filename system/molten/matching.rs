use crate::program::Symbol;
use crate::state::{State, Token};
use std::collections::HashSet;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Capture {
    Frame(Option<usize>),
    Environment(Box<State>),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Term {
    pub value: Symbol,
    pub capture: Option<Capture>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Slot {
    pub world: usize,
    pub token: Vec<usize>,
    pub position: usize,
}

pub fn pattern(
    input: &[Vec<Symbol>],
    capture: Option<usize>,
    environment: Option<&State>,
) -> Vec<Vec<Term>> {
    input
        .iter()
        .map(|particle| {
            particle
                .iter()
                .map(|&value| Term {
                    value,
                    capture: matches!(value, Symbol::Rule(_)).then(|| {
                        environment.map_or(Capture::Frame(capture), |environment| {
                            Capture::Environment(Box::new(environment.clone()))
                        })
                    }),
                })
                .collect()
        })
        .collect()
}

fn particle(
    pattern: &[Term],
    value: &[Token],
    state: &State,
    selected: &mut Vec<usize>,
    result: &mut Vec<Vec<usize>>,
) {
    let Some((head, tail)) = pattern.split_first() else {
        result.push(selected.clone());
        return;
    };
    let matches = |token: &Token| {
        token.value == head.value
            && match &head.capture {
                None => true,
                Some(Capture::Frame(capture)) => token.capture == *capture,
                Some(Capture::Environment(environment)) => token
                    .capture
                    .is_some_and(|capture| state.environment(capture) == **environment),
            }
    };
    let previous = selected
        .iter()
        .rev()
        .find(|&&id| value.iter().any(|token| token.id == id && matches(token)))
        .copied();
    for token in value {
        if !matches(token)
            || selected.contains(&token.id)
            || previous.is_some_and(|id| id >= token.id)
        {
            continue;
        }
        selected.push(token.id);
        particle(tail, value, state, selected, result);
        selected.pop();
    }
}

pub struct Gate {
    pattern: Vec<Vec<Term>>,
    candidate: Vec<Vec<Slot>>,
    prefix: Vec<Vec<Vec<Slot>>>,
    seen: Vec<HashSet<Vec<Slot>>>,
}

impl Gate {
    pub fn new(pattern: Vec<Vec<Term>>) -> Self {
        let count = pattern.len();
        let mut prefix = vec![Vec::new(); count + 1];
        prefix[0].push(Vec::new());
        Self {
            pattern,
            candidate: vec![Vec::new(); count],
            prefix,
            seen: vec![HashSet::new(); count + 1],
        }
    }

    fn extend(&mut self, binding: &[Slot], slot: Slot, result: &mut Vec<Vec<Slot>>) {
        if binding.iter().any(|item| item.world == slot.world) {
            return;
        }
        if binding
            .iter()
            .rev()
            .find(|item| self.pattern[item.position] == self.pattern[slot.position])
            .is_some_and(|item| item.world >= slot.world)
        {
            return;
        }
        let next = slot.position + 1;
        let mut binding = binding.to_vec();
        binding.push(slot);
        if !self.seen[next].insert(binding.clone()) {
            return;
        }
        self.prefix[next].push(binding.clone());
        if next == self.pattern.len() {
            result.push(binding);
            return;
        }
        for slot in self.candidate[next].clone() {
            self.extend(&binding, slot, result);
        }
    }

    pub fn arrive(&mut self, slot: Slot) -> Vec<Vec<Slot>> {
        let position = slot.position;
        if self.candidate[position].contains(&slot) {
            return Vec::new();
        }
        self.candidate[position].push(slot.clone());
        let mut result = Vec::new();
        for prefix in self.prefix[position].clone() {
            self.extend(&prefix, slot.clone(), &mut result);
        }
        result
    }
}

pub fn world(pattern: &[Vec<Term>], state: &State, frame: usize) -> Vec<Vec<Slot>> {
    if pattern.is_empty() {
        return vec![Vec::new()];
    }
    let mut gate = Gate::new(pattern.to_vec());
    let mut result = Vec::new();
    for (index, world) in state.world.iter().enumerate() {
        if world.frame != frame {
            continue;
        }
        for (position, pattern) in pattern.iter().enumerate() {
            let mut candidate = Vec::new();
            particle(
                pattern,
                &world.particle,
                state,
                &mut Vec::new(),
                &mut candidate,
            );
            for token in candidate {
                result.extend(gate.arrive(Slot {
                    world: index,
                    token,
                    position,
                }));
            }
        }
    }
    result
}
