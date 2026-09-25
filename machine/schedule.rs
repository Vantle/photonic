use crate::event::{Event, apply, enumerate};
use crate::exploration::fits;
use crate::flat::Flat;
use crate::limit::{Limit, Overflow};
use crate::state::State;

#[derive(Clone, Debug)]
pub struct Schedule {
    pub round: Vec<usize>,
    pub depth: usize,
    pub terminal: State,
}

impl Schedule {
    pub fn work(&self) -> usize {
        self.round.iter().sum()
    }
}

pub fn lineage(program: &Flat, level: &[usize], event: &[Event]) -> (Vec<usize>, usize) {
    let mut removed = vec![false; level.len()];
    let mut appended = Vec::new();
    let mut deepest = 0;
    for event in event {
        let depth = 1 + event
            .coherence
            .iter()
            .map(|&index| level[index])
            .max()
            .unwrap_or(0);
        deepest = deepest.max(depth);
        for &index in &event.coherence {
            removed[index] = true;
        }
        appended.extend(std::iter::repeat_n(
            depth,
            program.rule()[event.rule].output.len(),
        ));
    }
    let result = level
        .iter()
        .zip(&removed)
        .filter(|(_, removed)| !**removed)
        .map(|(level, _)| *level)
        .chain(appended)
        .collect();
    (result, deepest)
}

pub fn schedule(program: &Flat, initial: State, limit: &Limit) -> Result<Schedule, Overflow> {
    let mut level = vec![0; initial.coherence().len()];
    let mut state = initial;
    let mut round = Vec::new();
    let mut depth = 0;
    loop {
        let mut used = vec![false; state.coherence().len()];
        let mut chosen: Vec<Event> = Vec::new();
        let mut count = 0;
        enumerate(program, &state, |event| {
            count += 1;
            if count > limit.event {
                return false;
            }
            if event.coherence.iter().all(|&index| !used[index]) {
                for &index in &event.coherence {
                    used[index] = true;
                }
                chosen.push(event);
            }
            !used.iter().all(|&value| value)
        });
        if count > limit.event {
            return Err(Overflow::Event);
        }
        if chosen.is_empty() {
            return Ok(Schedule {
                round,
                depth,
                terminal: state,
            });
        }
        if round.len() >= limit.round {
            return Err(Overflow::Round);
        }
        round.push(chosen.len());
        let (next, deepest) = lineage(program, &level, &chosen);
        level = next;
        depth = depth.max(deepest);
        state = apply(program, &state, &chosen);
        if !fits(program, &state, limit) {
            return Err(Overflow::Size);
        }
    }
}
