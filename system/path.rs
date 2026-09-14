use crate::flow::Place;
use crate::obsidian::{Failure, Outcome};
use crate::program::Program;
use crate::runtime::{Limit, Runtime};
use crate::snapshot::Node;
use crate::source;
use crate::state::State;
use crate::support::Status;
use indexmap::IndexSet;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct Event {
    pub source: usize,
    pub target: usize,
    pub rule: String,
    pub footprint: Vec<Place>,
    pub exact: Vec<Place>,
    pub read: Vec<Place>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub outcome: Outcome,
    pub witness: Option<usize>,
    pub work: usize,
    pub program: source::Program,
    pub target: Vec<Vec<source::Value>>,
    pub state: Vec<Node>,
    pub event: Vec<Event>,
}

pub struct Summary {
    pub outcome: Outcome,
    pub witness: Option<Node>,
    pub event: usize,
    pub work: usize,
}

pub struct Search {
    program: source::Program,
    claim: Vec<Vec<source::Value>>,
    goal: State,
    runtime: Runtime,
    state: IndexSet<Arc<State>>,
    event: Vec<Event>,
    cursor: usize,
    work: usize,
    cycle: bool,
}

impl Search {
    pub fn new(program: source::Program, target: source::Program) -> Result<Self, Failure> {
        if !target.rule.is_empty() {
            return Err(Failure::Declaration);
        }
        let compiled = Program::new(program.clone());
        let initial = State::initial(&compiled);
        let mut goal = compiled.clone();
        goal.initial = goal.input(&target.initial);
        let goal = State::initial(&goal);
        let mut state = IndexSet::new();
        let initial = Arc::new(initial);
        state.insert(initial.clone());
        Ok(Self {
            program,
            claim: target.initial,
            goal,
            runtime: Runtime::seed(Arc::new(compiled), initial),
            state,
            event: Vec::new(),
            cursor: 0,
            work: 0,
            cycle: false,
        })
    }

    pub fn run(&mut self, budget: usize, limit: Limit) {
        for _ in 0..budget {
            if self.state[self.cursor].as_ref() == &self.goal || self.cycle {
                return;
            }
            if let Some(event) = self.runtime.first() {
                let next = self.runtime.state[event.target].clone();
                let known = self.state.get_index_of(&next);
                if known.is_none() && self.state.len() >= limit.state {
                    return;
                }
                if self.state.len() + self.event.len() + self.runtime.record() >= limit.record {
                    return;
                }
                let target = self.state.insert_full(next.clone()).0;
                self.event.push(Event {
                    source: self.cursor,
                    target,
                    rule: event.rule.to_owned(),
                    footprint: event.binding.footprint.iter().copied().collect(),
                    exact: event.binding.exact.iter().copied().collect(),
                    read: event.binding.read.iter().copied().collect(),
                });
                self.cursor = target;
                self.cycle = known.is_some();
                self.runtime = Runtime::seed(self.runtime.program.clone(), next);
                continue;
            }
            let retained = self.state.len() + self.event.len();
            let available = limit.record.saturating_sub(retained);
            let step = self.runtime.work;
            self.runtime.run(
                1,
                Some(Limit {
                    state: limit
                        .state
                        .saturating_sub(self.state.len())
                        .saturating_add(1)
                        .min(2),
                    record: available,
                    ..limit
                }),
            );
            self.work += self.runtime.work - step;
            if self.runtime.work == step {
                return;
            }
        }
    }

    pub fn summary(&self) -> Summary {
        let reached = self.state[self.cursor].as_ref() == &self.goal;
        Summary {
            outcome: if reached {
                Outcome::Reached
            } else {
                Outcome::Unknown
            },
            witness: reached.then(|| {
                Node::new(
                    self.cursor,
                    &self.state[self.cursor],
                    &self.runtime.program,
                    Status::Supported,
                )
            }),
            event: self.event.len(),
            work: self.work,
        }
    }

    pub fn report(&self) -> Report {
        let reached = self.state[self.cursor].as_ref() == &self.goal;
        Report {
            outcome: if reached {
                Outcome::Reached
            } else {
                Outcome::Unknown
            },
            witness: reached.then_some(self.cursor),
            work: self.work,
            program: self.program.clone(),
            target: self.claim.clone(),
            state: self
                .state
                .iter()
                .enumerate()
                .map(|(id, state)| Node::new(id, state, &self.runtime.program, Status::Supported))
                .collect(),
            event: self
                .event
                .iter()
                .map(|event| Event {
                    source: event.source,
                    target: event.target,
                    rule: event.rule.clone(),
                    footprint: event.footprint.clone(),
                    exact: event.exact.clone(),
                    read: event.read.clone(),
                })
                .collect(),
        }
    }
}
