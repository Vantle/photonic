use crate::event::{Event, apply, enumerate};
use crate::exploration::fits;
use crate::flat::Flat;
use crate::limit::Limit;
use crate::schedule::lineage;
use crate::state::State;
use code::hashing::Builder;
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct Walk {
    pub terminal: Option<State>,
    pub work: usize,
    pub depth: usize,
    pub cycle: bool,
    pub overflow: bool,
}

pub fn walk(
    program: &Flat,
    initial: State,
    limit: &Limit,
    mut choose: impl FnMut(usize) -> usize,
) -> Walk {
    let mut level = vec![0; initial.coherence().len()];
    let mut result = Walk {
        terminal: None,
        work: 0,
        depth: 0,
        cycle: false,
        overflow: false,
    };
    let Ok(key) = initial.key(limit.individualization) else {
        result.overflow = true;
        return result;
    };
    let mut seen: HashSet<_, Builder> = HashSet::from_iter([key]);
    let mut state = initial;
    loop {
        let mut event: Vec<Event> = Vec::new();
        let complete = enumerate(program, &state, |candidate| {
            event.push(candidate);
            event.len() <= limit.event
        });
        if !complete {
            result.overflow = true;
            return result;
        }
        if event.is_empty() {
            result.terminal = Some(state);
            return result;
        }
        if result.work >= limit.configuration || limit.expired() {
            result.overflow = true;
            return result;
        }
        let chosen = event.swap_remove(choose(event.len()));
        let (next, deepest) = lineage(program, &level, std::slice::from_ref(&chosen));
        level = next;
        result.depth = result.depth.max(deepest);
        result.work += 1;
        state = apply(program, &state, &[chosen]);
        if !fits(program, &state, limit) {
            result.overflow = true;
            return result;
        }
        let Ok(key) = state.key(limit.individualization) else {
            result.overflow = true;
            return result;
        };
        if !seen.insert(key) {
            result.cycle = true;
            return result;
        }
    }
}
