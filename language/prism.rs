use crate::executor::Executor;
use crate::runtime::{Limit, Runtime};
use crate::snapshot::Snapshot;
use crate::source;
use crate::state::State;
use crate::support::Status;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Reached,
    Unreachable,
    Unknown,
}

#[derive(Debug, Serialize)]
pub struct Report<Execution = Snapshot, Program = source::Program, Target = source::Program> {
    pub outcome: Outcome,
    pub witness: Option<usize>,
    pub program: Program,
    pub target: Target,
    pub execution: Execution,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct Verdict {
    pub outcome: Outcome,
    pub witness: Option<usize>,
}

pub struct Search {
    runtime: Runtime,
    target: State,
    program: source::Program,
    claim: source::Program,
}

fn configuration(runtime: &Runtime, source: &source::Program) -> State {
    let target = runtime.program.target(source);
    State::configuration(&target.initial, &target.rule)
}

impl Search {
    pub fn new(program: source::Program, target: source::Program) -> Self {
        let runtime = Runtime::new(&program);
        Self {
            target: configuration(&runtime, &target),
            runtime,
            program,
            claim: target,
        }
    }

    pub fn target(&mut self, target: source::Program) {
        self.target = configuration(&self.runtime, &target);
        self.claim = target;
    }

    pub fn run(&mut self, budget: usize, limit: Option<Limit>) {
        self.runtime.run(budget, limit);
    }

    pub fn parallel(&mut self, executor: &Executor, budget: usize, limit: Option<Limit>) {
        self.runtime.parallel(executor, budget, limit);
    }

    pub fn verdict(&self) -> Verdict {
        let candidate = self.runtime.state.get_index_of(&self.target);
        let status = candidate.map(|index| self.runtime.status(index));
        let outcome = match status {
            Some(Status::Supported) => Outcome::Reached,
            _ if self.runtime.closed() => Outcome::Unreachable,
            _ => Outcome::Unknown,
        };
        Verdict {
            outcome,
            witness: candidate.filter(|_| outcome == Outcome::Reached),
        }
    }

    pub fn view(&self) -> impl Serialize + '_ {
        let verdict = self.verdict();
        Report {
            outcome: verdict.outcome,
            witness: verdict.witness,
            program: &self.program,
            target: &self.claim,
            execution: self.runtime.view(),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        self.runtime.snapshot()
    }

    pub fn report(&self) -> Report {
        let verdict = self.verdict();
        Report {
            outcome: verdict.outcome,
            witness: verdict.witness,
            program: self.program.clone(),
            target: self.claim.clone(),
            execution: self.runtime.snapshot(),
        }
    }
}
