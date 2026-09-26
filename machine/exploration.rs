use crate::event::{apply, enumerate};
use crate::flat::Flat;
use crate::limit::Limit;
use crate::state::State;
use code::atom::Atom;
use code::canonical::Key;
use hashing::Builder;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug)]
pub struct Exploration {
    pub terminal: Vec<State>,
    pub state: usize,
    pub cycle: bool,
    pub overflow: bool,
    pub truncated: bool,
}

impl Exploration {
    pub fn complete(&self) -> bool {
        !self.cycle && !self.overflow && !self.truncated
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Mark {
    Open,
    Closed,
}

struct Frame {
    index: usize,
    successor: Vec<(State, Key<Atom>)>,
    cursor: usize,
}

pub fn fits(program: &Flat, state: &State, limit: &Limit) -> bool {
    state.coherence().len() <= limit.coherence
        && state.size() + program.rule().len() <= limit.occurrence
}

pub fn successor(program: &Flat, state: &State, limit: &Limit) -> Option<Vec<(State, Key<Atom>)>> {
    let mut result: Vec<(State, Key<Atom>)> = Vec::new();
    let mut known: HashSet<Key<Atom>, Builder> = HashSet::default();
    let mut count = 0;
    let complete = enumerate(program, state, |event| {
        count += 1;
        if count > limit.event {
            return false;
        }
        let next = apply(program, state, &[event]);
        if !fits(program, &next, limit) {
            return false;
        }
        let Ok(identity) = next.key(limit.individualization) else {
            return false;
        };
        if known.insert(identity.clone()) {
            result.push((next, identity));
        }
        true
    });
    complete.then_some(result)
}

pub fn explore(
    program: &Flat,
    initial: State,
    limit: &Limit,
    mut stop: impl FnMut(&State) -> bool,
) -> Exploration {
    let mut exploration = Exploration {
        terminal: Vec::new(),
        state: 1,
        cycle: false,
        overflow: false,
        truncated: false,
    };
    let mut mark = vec![Mark::Open];
    let Ok(key) = initial.key(limit.individualization) else {
        exploration.overflow = true;
        return exploration;
    };
    let mut index: HashMap<Key<Atom>, usize, Builder> = HashMap::from_iter([(key, 0)]);
    let Some(first) = successor(program, &initial, limit) else {
        exploration.overflow = true;
        return exploration;
    };
    if first.is_empty() {
        exploration.terminal.push(initial);
        return exploration;
    }
    let mut stack = vec![Frame {
        index: 0,
        successor: first,
        cursor: 0,
    }];
    while let Some(frame) = stack.last_mut() {
        if frame.cursor == frame.successor.len() {
            mark[frame.index] = Mark::Closed;
            stack.pop();
            continue;
        }
        let position = frame.cursor;
        frame.cursor += 1;
        let identity = frame.successor[position].1.clone();
        if let Some(&known) = index.get(&identity) {
            if mark[known] == Mark::Open {
                exploration.cycle = true;
                return exploration;
            }
            continue;
        }
        if exploration.state >= limit.configuration || limit.expired() {
            exploration.overflow = true;
            return exploration;
        }
        let next = std::mem::take(&mut frame.successor[position].0);
        let Some(following) = successor(program, &next, limit) else {
            exploration.overflow = true;
            return exploration;
        };
        let target = mark.len();
        index.insert(identity, target);
        exploration.state += 1;
        if following.is_empty() {
            mark.push(Mark::Closed);
            let halt = stop(&next);
            exploration.terminal.push(next);
            if halt || exploration.terminal.len() > limit.terminal {
                exploration.truncated = true;
                return exploration;
            }
            continue;
        }
        mark.push(Mark::Open);
        stack.push(Frame {
            index: target,
            successor: following,
            cursor: 0,
        });
    }
    exploration
}
