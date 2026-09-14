use crate::executor::Executor;
use crate::runtime::{Limit, Runtime};
use crate::snapshot::Snapshot;
use crate::source::{self, Value};
use crate::state::State;
use crate::support::Status;
use miette::Diagnostic;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Diagnostic, Error)]
pub enum Failure {
    #[error("an Prism target contains a configuration, without additional declarations")]
    #[diagnostic(code(photonic::prism::declaration))]
    Declaration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Reached,
    Unreachable,
    Unknown,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub outcome: Outcome,
    pub witness: Option<usize>,
    pub program: source::Program,
    pub target: Vec<Vec<Value>>,
    pub execution: Snapshot,
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
    claim: Vec<Vec<Value>>,
}

impl Search {
    pub fn new(program: source::Program, target: source::Program) -> Result<Self, Failure> {
        if !target.rule.is_empty() {
            return Err(Failure::Declaration);
        }
        let runtime = Runtime::new(program.clone());
        let mut compiled = runtime.program.as_ref().clone();
        compiled.initial = compiled.input(&target.initial);
        let configuration = State::initial(&compiled);
        Ok(Self {
            runtime,
            target: configuration,
            program,
            claim: target.initial,
        })
    }

    pub fn target(&mut self, target: source::Program) -> Result<(), Failure> {
        if !target.rule.is_empty() {
            return Err(Failure::Declaration);
        }
        let mut compiled = self.runtime.program.as_ref().clone();
        compiled.initial = compiled.input(&target.initial);
        self.target = State::initial(&compiled);
        self.claim = target.initial;
        Ok(())
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
