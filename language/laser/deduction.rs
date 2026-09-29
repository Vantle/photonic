use super::Laser;
use crate::profile;

// An inferred event's deduction is the walk its match took back to its source: the events from its
// source to the configuration where the rule matched. Each carried trace remembers the crossing
// that first brought it, so a walk is read by following those crossings from a trace that
// identifies the event until a trace the configuration found itself; of the traces that identify
// an event, the one with the shortest walk is read. Direct events have none.
#[derive(Default)]
pub(super) struct Deduction {
    offset: Vec<usize>,
    step: Vec<u32>,
}

fn walk(laser: &Laser, mut state: usize, mut position: usize) -> Vec<u32> {
    let mut step = Vec::new();
    while position >= laser.origin[state] {
        let (event, from) = laser.parent[state][position - laser.origin[state]];
        step.push(event);
        state = laser.event[event as usize].target;
        position = from as usize;
    }
    step
}

impl Deduction {
    pub fn derive(laser: &Laser) -> Self {
        let _scope = profile::Scope::new(profile::Phase::Deduction);
        let mut shortest = vec![None::<Vec<u32>>; laser.event.len()];
        for (state, list) in laser.link.iter().enumerate() {
            for position in 0..list.len() {
                let Some(event) = laser
                    .linked(state, position)
                    .filter(|&event| !laser.event[event].direct)
                else {
                    continue;
                };
                let step = walk(laser, state, position);
                if shortest[event]
                    .as_ref()
                    .is_none_or(|known| step.len() < known.len())
                {
                    shortest[event] = Some(step);
                }
            }
        }
        let mut deduction = Self {
            offset: Vec::with_capacity(laser.event.len() + 1),
            step: Vec::new(),
        };
        deduction.offset.push(0);
        for step in shortest {
            deduction.step.extend(step.unwrap_or_default());
            deduction.offset.push(deduction.step.len());
        }
        deduction
    }

    pub fn get(&self, event: usize) -> Vec<usize> {
        self.step[self.offset[event]..self.offset[event + 1]]
            .iter()
            .map(|&event| event as usize)
            .collect()
    }
}
