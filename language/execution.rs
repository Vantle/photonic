use crate::hashing::Builder;
use crate::program::{Program, Symbol};
use crate::reduction::{Event, Search};
use crate::runtime::Limit;
use crate::source;
use crate::state::State;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct Bound {
    pub state: usize,
    pub terminal: usize,
    pub step: usize,
    pub work: usize,
}

impl Default for Bound {
    fn default() -> Self {
        Self {
            state: 4_096,
            terminal: 4,
            step: 4_096,
            work: 1_000_000,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Occurrence {
    pub id: usize,
    pub value: source::Value,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub struct Observation {
    pub coherence: Vec<Vec<Occurrence>>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Exploration {
    pub terminal: Vec<Observation>,
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

#[derive(Clone, Debug, Default, Serialize)]
pub struct Walk {
    pub terminal: Option<Observation>,
    pub work: usize,
    pub depth: usize,
    pub cycle: bool,
    pub overflow: bool,
}

fn value(program: &Program, symbol: Symbol) -> source::Value {
    match symbol {
        Symbol::Atom(index) => source::Value::Atom(program.atom[index].clone()),
        Symbol::Rule(index) => source::Value::Rule {
            rule: Box::new(definition(program, index)),
        },
    }
}

fn definition(program: &Program, index: usize) -> source::Definition {
    let instruction = &program.rule[index];
    source::Definition {
        name: String::new(),
        input: instruction
            .input
            .iter()
            .map(|particle| {
                particle
                    .iter()
                    .map(|&symbol| value(program, symbol))
                    .collect()
            })
            .collect(),
        rest: Vec::new(),
        output: instruction
            .output
            .iter()
            .map(|output| source::Output {
                particle: output
                    .particle
                    .iter()
                    .map(|&symbol| value(program, symbol))
                    .collect(),
                body: output.body.map(|scope| {
                    program.scope[scope]
                        .rule
                        .iter()
                        .map(|&rule| definition(program, rule))
                        .collect()
                }),
            })
            .collect(),
    }
}

fn observation(program: &Program, state: &State) -> Observation {
    Observation {
        coherence: state
            .world
            .iter()
            .filter(|world| world.frame == 0)
            .map(|world| {
                world
                    .particle
                    .iter()
                    .map(|token| Occurrence {
                        id: token.id,
                        value: value(program, token.value),
                    })
                    .collect()
            })
            .collect(),
    }
}

fn successor(
    program: &Arc<Program>,
    state: &Arc<State>,
    limit: Limit,
    bound: &Bound,
) -> Option<Vec<Event>> {
    let mut search = Search::new(program.clone(), state.clone());
    let mut result = Vec::new();
    loop {
        let work = search.work;
        match search.run(limit) {
            Some(event) => result.push(event),
            None if search.work == work => break,
            None if search.work > bound.work => return None,
            None => {}
        }
    }
    (search.deferred() == 0).then_some(result)
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Mark {
    Open,
    Closed,
}

struct Frame {
    index: usize,
    successor: Vec<Event>,
    cursor: usize,
}

pub fn explore(
    source: &source::Program,
    limit: Limit,
    bound: Bound,
    mut stop: impl FnMut(&Observation) -> bool,
) -> Exploration {
    let program = Arc::new(Program::new(source));
    let initial = Arc::new(State::initial(&program));
    let mut exploration = Exploration {
        state: 1,
        ..Exploration::default()
    };
    let mut index: HashMap<State, usize, Builder> = HashMap::default();
    index.insert(initial.canonical().state, 0);
    let mut mark = vec![Mark::Open];
    let Some(first) = successor(&program, &initial, limit, &bound) else {
        exploration.overflow = true;
        return exploration;
    };
    if first.is_empty() {
        exploration.terminal.push(observation(&program, &initial));
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
        let state = frame.successor[frame.cursor].state.clone();
        frame.cursor += 1;
        let canonical = state.canonical().state;
        if let Some(&known) = index.get(&canonical) {
            if mark[known] == Mark::Open {
                exploration.cycle = true;
                return exploration;
            }
            continue;
        }
        if exploration.state >= bound.state {
            exploration.overflow = true;
            return exploration;
        }
        let Some(following) = successor(&program, &state, limit, &bound) else {
            exploration.overflow = true;
            return exploration;
        };
        let target = mark.len();
        index.insert(canonical, target);
        exploration.state += 1;
        if following.is_empty() {
            mark.push(Mark::Closed);
            let terminal = observation(&program, &state);
            let halt = stop(&terminal);
            exploration.terminal.push(terminal);
            if halt || exploration.terminal.len() > bound.terminal {
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

pub fn walk(
    source: &source::Program,
    limit: Limit,
    bound: Bound,
    mut choose: impl FnMut(usize) -> usize,
) -> Walk {
    let program = Arc::new(Program::new(source));
    let mut state = Arc::new(State::initial(&program));
    let mut level = vec![0usize; state.world.len()];
    let mut seen: HashSet<State, Builder> = HashSet::default();
    seen.insert(state.canonical().state);
    let mut result = Walk::default();
    loop {
        let Some(mut event) = successor(&program, &state, limit, &bound) else {
            result.overflow = true;
            return result;
        };
        if event.is_empty() {
            result.terminal = Some(observation(&program, &state));
            return result;
        }
        if result.work >= bound.step {
            result.overflow = true;
            return result;
        }
        let chosen = event.swap_remove(choose(event.len()));
        let depth = 1 + chosen
            .change
            .world
            .iter()
            .map(|&world| level[world])
            .max()
            .unwrap_or(0);
        level = level
            .iter()
            .enumerate()
            .filter(|(world, _)| !chosen.change.world.contains(world))
            .map(|(_, level)| *level)
            .chain(std::iter::repeat_n(depth, chosen.change.insertion.len()))
            .collect();
        result.depth = result.depth.max(depth);
        result.work += 1;
        state = chosen.state;
        if !seen.insert(state.canonical().state) {
            result.cycle = true;
            return result;
        }
    }
}

#[cfg(test)]
#[path = "test/execution.rs"]
mod test;
