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
    #[error("an Obsidian target contains a configuration, without additional declarations")]
    #[diagnostic(code(molten::obsidian::declaration))]
    Declaration,
    #[error(
        "an Obsidian target requires concrete values; pattern variables are not quantified claims"
    )]
    #[diagnostic(code(molten::obsidian::variable))]
    Variable,
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

pub struct Search {
    runtime: Runtime,
    target: State,
    program: source::Program,
    claim: Vec<Vec<Value>>,
}

fn concrete(value: &Value) -> bool {
    match value {
        Value::Variable { .. } => false,
        Value::Structure { particle, .. } => particle.iter().all(concrete),
        Value::Atom(_) | Value::Rule { .. } => true,
    }
}

impl Search {
    pub fn new(program: source::Program, target: source::Program) -> Result<Self, Failure> {
        if !target.rule.is_empty() {
            return Err(Failure::Declaration);
        }
        if !target.initial.iter().flatten().all(concrete) {
            return Err(Failure::Variable);
        }
        let runtime = Runtime::new(program.clone());
        let mut compiled = runtime.program.clone();
        compiled.initial = compiled.input(&target.initial);
        let configuration = State::initial(&compiled);
        Ok(Self {
            runtime,
            target: configuration,
            program,
            claim: target.initial,
        })
    }

    pub fn run(&mut self, steps: usize, limit: Option<Limit>) {
        self.runtime.run(steps, limit);
    }

    pub fn parallel(&mut self, executor: &Executor, steps: usize, limit: Option<Limit>) {
        self.runtime.parallel(executor, steps, limit);
    }

    pub fn report(&self) -> Report {
        let execution = self.runtime.snapshot();
        let candidate = self
            .runtime
            .state
            .iter()
            .position(|state| state.as_ref() == &self.target);
        let status = candidate.map(|index| execution.state[index].status);
        let outcome = match status {
            Some(Status::Supported) => Outcome::Reached,
            Some(Status::Conditional) => Outcome::Unknown,
            _ if execution.closed => Outcome::Unreachable,
            _ => Outcome::Unknown,
        };
        Report {
            outcome,
            witness: candidate.filter(|_| outcome == Outcome::Reached),
            program: self.program.clone(),
            target: self.claim.clone(),
            execution,
        }
    }
}
