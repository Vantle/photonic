use super::{CHUNK, Laser, Link, map};
use crate::executor::Executor;

// Support is the least set closed under the interpreter's clauses, read over traces instead of
// views: the initial configuration and every configuration's own matches are given; a trace
// carried across a supported event from a supported trace is supported; an event is supported
// when its source is and a supported trace there identifies it; a configuration is supported
// when a supported event reaches it. Each round takes what the round before it supported and
// finds in parallel what that supports, given everything supported so far; a clause with two
// premises is found by whichever premise enters last, since each round sees all the rounds before
// it, so the rounds reach the same least set as any order would. Each trace, event and
// configuration enters a round once, so establishing support is linear in the traces and their
// crossings.
pub(super) struct Support {
    pub state: Vec<bool>,
    pub event: Vec<bool>,
}

#[derive(Clone, Copy)]
enum Item {
    State(usize),
    Event(usize),
    Trace(usize, usize),
}

struct Mark {
    state: Vec<bool>,
    event: Vec<bool>,
    trace: Vec<Vec<bool>>,
}

fn identify(laser: &Laser, state: usize, position: usize) -> Option<usize> {
    match laser.link[state].get(position)? {
        Link::Absent => None,
        Link::Event(event) => Some(*event),
        Link::Unresolved => laser.find(state, &laser.trace[state][position]),
    }
}

fn landing(laser: &Laser, event: usize, position: usize) -> Option<usize> {
    let found = laser.crossed[event].get(position).copied().flatten()?;
    Some(found.get() as usize - 1)
}

impl Mark {
    fn admit(&mut self, item: Item) -> bool {
        let slot = match item {
            Item::State(index) => &mut self.state[index],
            Item::Event(value) => &mut self.event[value],
            Item::Trace(index, position) => &mut self.trace[index][position],
        };
        !std::mem::replace(slot, true)
    }

    fn follow(&self, laser: &Laser, item: Item, found: &mut Vec<Item>) {
        match item {
            Item::State(index) => {
                for (position, &supported) in self.trace[index].iter().enumerate() {
                    if supported
                        && let Some(value) = identify(laser, index, position)
                        && !self.event[value]
                    {
                        found.push(Item::Event(value));
                    }
                }
            }
            Item::Event(value) => {
                let (source, target) = (laser.event[value].source, laser.event[value].target);
                if !self.state[target] {
                    found.push(Item::State(target));
                }
                for (position, &supported) in self.trace[target].iter().enumerate() {
                    if supported
                        && let Some(landed) = landing(laser, value, position)
                        && !self.trace[source][landed]
                    {
                        found.push(Item::Trace(source, landed));
                    }
                }
            }
            Item::Trace(index, position) => {
                if self.state[index]
                    && let Some(value) = identify(laser, index, position)
                    && !self.event[value]
                {
                    found.push(Item::Event(value));
                }
                for &value in &laser.incoming[index] {
                    let source = laser.event[value].source;
                    if self.event[value]
                        && let Some(landed) = landing(laser, value, position)
                        && !self.trace[source][landed]
                    {
                        found.push(Item::Trace(source, landed));
                    }
                }
            }
        }
    }
}

pub(super) fn establish(laser: &Laser, executor: Option<&Executor>) -> Support {
    let _scope = crate::profile::Scope::new(crate::profile::Phase::Support);
    let mut mark = Mark {
        state: vec![false; laser.state.len()],
        event: vec![false; laser.event.len()],
        trace: laser
            .trace
            .iter()
            .map(|set| vec![false; set.len()])
            .collect(),
    };
    let mut frontier = vec![Item::State(0)];
    mark.state[0] = true;
    for (index, &origin) in laser.origin.iter().enumerate() {
        mark.trace[index][..origin].fill(true);
        frontier.extend((0..origin).map(|position| Item::Trace(index, position)));
    }
    while !frontier.is_empty() {
        let chunk = frontier.chunks(CHUNK).map(<[Item]>::to_vec).collect();
        let found = map(executor, chunk, |chunk: Vec<Item>| {
            let mut found = Vec::new();
            for item in chunk {
                mark.follow(laser, item, &mut found);
            }
            found
        });
        frontier = found
            .into_iter()
            .flatten()
            .filter(|&item| mark.admit(item))
            .collect();
    }
    Support {
        state: mark.state,
        event: mark.event,
    }
}
