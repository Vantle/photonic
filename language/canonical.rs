use crate::profile;
use crate::state::{Canonical, State};
use division::Division;
use std::sync::Arc;
use thiserror::Error;
use whole::Whole;

mod division;
mod extraction;
mod renaming;
mod whole;

// A search that ran out of budget before it named its configuration.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("naming the configuration takes more steps than its budget allows")]
pub(crate) struct Exhausted;

// A configuration that shares no token, as every program's initial configuration and every target
// does, divides down to single scopes and coherences, so its search ends after a number of steps
// polynomial in its size and it is named with this budget.
pub(crate) const UNLIMITED: usize = usize::MAX;

// A configuration whose whole search takes at most this many steps keeps the name that search
// gives it; a larger one with interchangeable parts is named by its parts instead.
const LIMIT: usize = 64;

// A configuration's canonical form, found a step at a time. A step renames one candidate
// ordering, orders the coherences for the frame orderings under them, divides the configuration
// into parts, or names it from its named parts, so every step is polynomial in its size.
pub(crate) struct Search {
    size: usize,
    stage: Stage,
}

enum Stage {
    Whole(Whole),
    Division(Box<Division>),
    Complete(Option<Canonical>),
}

impl Search {
    pub fn new(state: Arc<State>) -> Self {
        Self {
            size: state.world.len(),
            stage: Stage::Whole(Whole::new(state)),
        }
    }

    pub(crate) fn cost(&self) -> usize {
        self.size
    }

    pub fn step(&mut self) -> bool {
        let _scope = profile::Scope::new(profile::Phase::Canonicalization);
        self.advance()
    }

    // A step, timed by the caller's scope, so the searches of parts do not count twice.
    fn advance(&mut self) -> bool {
        let named = match &mut self.stage {
            Stage::Complete(_) => return true,
            Stage::Division(division) => division.step(),
            Stage::Whole(whole) => {
                let fresh = whole.fresh();
                if !whole.step() {
                    if fresh && whole.cost().is_some_and(|cost| cost > LIMIT) {
                        self.divide();
                    }
                    return false;
                }
                let Stage::Whole(whole) = std::mem::replace(&mut self.stage, Stage::Complete(None))
                else {
                    unreachable!("the search was whole")
                };
                whole.finish()
            }
        };
        let Some(named) = named else {
            return false;
        };
        self.stage = Stage::Complete(Some(named));
        true
    }

    // Names the configuration by its parts from the next step on, when it has two or more.
    fn divide(&mut self) {
        let Stage::Whole(whole) = &self.stage else {
            return;
        };
        if let Some(division) = Division::new(whole.state.clone(), whole.refinement()) {
            self.stage = Stage::Division(Box::new(division));
        }
    }

    pub fn finish(self) -> Option<Canonical> {
        match self.stage {
            Stage::Complete(named) => named,
            Stage::Whole(_) | Stage::Division(_) => None,
        }
    }
}

#[cfg(test)]
#[path = "test/canonical.rs"]
mod test;
