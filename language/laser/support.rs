use super::{CHUNK, Laser, map};
use crate::executor::Executor;
use crate::profile;

// Support is the least set closed under the interpreter's clauses, read over traces instead of
// views: the initial configuration and every configuration's own matches are given; a trace
// carried across a supported event from a supported trace is supported; an event is supported
// when its source is and a supported trace there identifies it; a configuration is supported
// when a supported event reaches it. Each round takes what the round before it supported and
// finds in parallel what that supports, given everything supported so far; a clause with two
// premises is found by whichever premise enters last, since each round sees all the rounds before
// it, so the rounds reach the same least set as any order would. Each trace, event and
// configuration enters a round once and reads only the crossings recorded for it and the events
// that recorded any, so a plain exploration, which carries nothing, reads no crossing at all.
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

// What is supported so far, and for each configuration the incoming events that carried any of
// its traces.
struct Mark {
    state: Vec<bool>,
    event: Vec<bool>,
    trace: Vec<Vec<bool>>,
    carrier: Vec<Vec<usize>>,
}

impl Mark {
    fn admit(&mut self, item: Item) -> bool {
        let slot = match item {
            Item::State(state) => &mut self.state[state],
            Item::Event(event) => &mut self.event[event],
            Item::Trace(state, position) => &mut self.trace[state][position],
        };
        !std::mem::replace(slot, true)
    }

    // A supported trace at a supported configuration supports the event it identifies.
    fn identify(&self, laser: &Laser, state: usize, position: usize, found: &mut Vec<Item>) {
        if let Some(event) = laser.linked(state, position)
            && !self.event[event]
        {
            found.push(Item::Event(event));
        }
    }

    // A supported trace carried across a supported event supports the trace it lands on.
    fn carry(&self, laser: &Laser, event: usize, position: usize, found: &mut Vec<Item>) {
        let source = laser.event[event].source;
        if let Some(landed) = laser.landing(event, position)
            && !self.trace[source][landed]
        {
            found.push(Item::Trace(source, landed));
        }
    }

    fn follow(&self, laser: &Laser, item: Item, found: &mut Vec<Item>) {
        match item {
            Item::State(state) => {
                for (position, &supported) in self.trace[state].iter().enumerate() {
                    if supported {
                        self.identify(laser, state, position, found);
                    }
                }
            }
            Item::Event(event) => {
                let target = laser.event[event].target;
                if !self.state[target] {
                    found.push(Item::State(target));
                }
                for position in 0..laser.crossed[event].len() {
                    if self.trace[target][position] {
                        self.carry(laser, event, position, found);
                    }
                }
            }
            Item::Trace(state, position) => {
                if self.state[state] {
                    self.identify(laser, state, position, found);
                }
                for &event in &self.carrier[state] {
                    if self.event[event] {
                        self.carry(laser, event, position, found);
                    }
                }
            }
        }
    }
}

pub(super) fn establish(laser: &Laser, executor: Option<&Executor>) -> Support {
    let _scope = profile::Scope::new(profile::Phase::Support);
    let carrier = map(executor, (0..laser.state.len()).collect(), |state| {
        laser.incoming[state]
            .iter()
            .copied()
            .filter(|&event| !laser.crossed[event].is_empty())
            .collect::<Vec<_>>()
    });
    let mut mark = Mark {
        state: vec![false; laser.state.len()],
        event: vec![false; laser.event.len()],
        trace: laser
            .trace
            .iter()
            .map(|set| vec![false; set.len()])
            .collect(),
        carrier,
    };
    let mut frontier = vec![Item::State(0)];
    mark.state[0] = true;
    for (state, &origin) in laser.origin.iter().enumerate() {
        mark.trace[state][..origin].fill(true);
        frontier.extend((0..origin).map(|position| Item::Trace(state, position)));
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
