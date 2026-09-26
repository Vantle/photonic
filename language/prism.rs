use crate::program::Program;
use crate::state::State;
use crate::status::Status;
use frontend::source;
use indexmap::IndexSet;
use serde::Serialize;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Reached,
    Unreachable,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct Verdict {
    pub outcome: Outcome,
    pub witness: Option<usize>,
}

pub struct Reach {
    program: Arc<Program>,
    state: IndexSet<Arc<State>, hashing::Builder>,
    status: Vec<Status>,
    closed: bool,
}

pub(crate) fn verdict(
    program: &Program,
    state: &IndexSet<Arc<State>, hashing::Builder>,
    status: impl Fn(usize) -> Status,
    closed: bool,
    target: &source::Program,
) -> Verdict {
    let Some(target) = State::target(program, target) else {
        return Verdict {
            outcome: Outcome::Unknown,
            witness: None,
        };
    };
    let candidate = state.get_index_of(&target);
    let outcome = match candidate.map(status) {
        Some(Status::Supported) => Outcome::Reached,
        _ if closed => Outcome::Unreachable,
        _ => Outcome::Unknown,
    };
    Verdict {
        outcome,
        witness: candidate.filter(|_| outcome == Outcome::Reached),
    }
}

impl Reach {
    pub(crate) fn new(
        program: Arc<Program>,
        state: IndexSet<Arc<State>, hashing::Builder>,
        status: Vec<Status>,
        closed: bool,
    ) -> Self {
        Self {
            program,
            state,
            status,
            closed,
        }
    }

    pub fn verdict(&self, target: &source::Program) -> Verdict {
        verdict(
            &self.program,
            &self.state,
            |index| self.status[index],
            self.closed,
            target,
        )
    }
}
