use super::{Laser, Link};

fn fire(value: usize, event: &mut [bool], work: &mut Vec<Item>) {
    if !std::mem::replace(&mut event[value], true) {
        work.push(Item::Event(value));
    }
}

// Support is the least set closed under the interpreter's clauses, read over traces instead of
// views: the initial configuration and every configuration's own matches are given; a trace
// carried across a supported event from a supported trace is supported; an event is supported
// when its source is and a supported trace there identifies it; a configuration is supported
// when a supported event reaches it. Each trace, event and configuration enters the work once,
// so establishing support is linear in the traces and their crossings.
pub(super) struct Support {
    pub state: Vec<bool>,
    pub event: Vec<bool>,
}

enum Item {
    State(usize),
    Event(usize),
    Trace(usize, usize),
}

fn identify(laser: &Laser, state: usize, position: usize) -> Option<usize> {
    match laser.link[state].get(position)? {
        Link::Absent => None,
        Link::Event(event) => Some(*event),
        Link::Unresolved => laser.find(state, &laser.trace[state][position]),
    }
}

pub(super) fn establish(laser: &Laser) -> Support {
    let mut state = vec![false; laser.state.len()];
    let mut event = vec![false; laser.event.len()];
    let mut trace = laser
        .trace
        .iter()
        .map(|set| vec![false; set.len()])
        .collect::<Vec<_>>();
    let mut work = vec![Item::State(0)];
    state[0] = true;
    for (index, &origin) in laser.origin.iter().enumerate() {
        trace[index][..origin].fill(true);
        work.extend((0..origin).map(|position| Item::Trace(index, position)));
    }
    while let Some(item) = work.pop() {
        match item {
            Item::State(index) => {
                let supported = trace[index]
                    .iter()
                    .enumerate()
                    .filter(|&(_, &supported)| supported)
                    .filter_map(|(position, _)| identify(laser, index, position))
                    .collect::<Vec<_>>();
                for value in supported {
                    fire(value, &mut event, &mut work);
                }
            }
            Item::Event(value) => {
                let (source, target) = (laser.event[value].source, laser.event[value].target);
                if !std::mem::replace(&mut state[target], true) {
                    work.push(Item::State(target));
                }
                for position in 0..trace[target].len() {
                    let landing = laser.crossed[value].get(position).copied().flatten();
                    if trace[target][position]
                        && let Some(landing) = landing
                    {
                        let found = landing.get() as usize - 1;
                        if !std::mem::replace(&mut trace[source][found], true) {
                            work.push(Item::Trace(source, found));
                        }
                    }
                }
            }
            Item::Trace(index, position) => {
                if state[index]
                    && let Some(value) = identify(laser, index, position)
                {
                    fire(value, &mut event, &mut work);
                }
                for &value in &laser.incoming[index] {
                    let landing = laser.crossed[value].get(position).copied().flatten();
                    if event[value]
                        && let Some(landing) = landing
                    {
                        let source = laser.event[value].source;
                        let found = landing.get() as usize - 1;
                        if !std::mem::replace(&mut trace[source][found], true) {
                            work.push(Item::Trace(source, found));
                        }
                    }
                }
            }
        }
    }
    Support { state, event }
}
