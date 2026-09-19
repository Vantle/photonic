use crate::flow::Place;
use crate::prism::{Failure, Outcome};
use crate::program::Program;
use crate::runtime::Limit;
use crate::snapshot::Node;
use crate::source;
use crate::state::{Canonical, State};
use crate::support::Status;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

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

#[derive(Debug, Serialize)]
pub struct Statistic {
    pub refinement: usize,
    pub resolution: usize,
    pub normalization: usize,
    pub bucket: usize,
    pub preparation: usize,
    pub reuse: usize,
}

struct Record {
    state: Arc<State>,
    canonical: OnceLock<Canonical>,
    signature: OnceLock<u64>,
    resolution: OnceLock<u64>,
    normalization: Option<crate::canonical::Search>,
}

impl Record {
    fn new(state: Arc<State>) -> Self {
        Self {
            state,
            canonical: OnceLock::new(),
            signature: OnceLock::new(),
            resolution: OnceLock::new(),
            normalization: None,
        }
    }

    fn signature(&self) -> u64 {
        *self
            .signature
            .get_or_init(|| crate::fingerprint::signature(&self.state))
    }

    fn compatible(&self, other: &Self) -> bool {
        self.signature() == other.signature()
            && self
                .resolution
                .get_or_init(|| crate::fingerprint::resolution(&self.state))
                == other
                    .resolution
                    .get_or_init(|| crate::fingerprint::resolution(&other.state))
    }

    fn canonical(&self) -> &Canonical {
        self.canonical.get_or_init(|| self.state.canonical())
    }

    fn advance(&mut self) {
        let search = self
            .normalization
            .get_or_insert_with(|| crate::canonical::Search::new(self.state.clone()));
        if search.step() {
            let canonical = self.normalization.take().unwrap().finish().unwrap();
            self.canonical.get_or_init(|| canonical);
        }
    }
}

struct Step {
    source: usize,
    target: usize,
    rule: usize,
    binding: crate::flow::Binding,
    event: OnceLock<Event>,
}

pub struct Search {
    program: source::Program,
    compiled: Arc<Program>,
    claim: Vec<Vec<source::Value>>,
    goal: Record,
    signature: u64,
    runtime: crate::reduction::Search,
    state: Vec<Record>,
    index: HashMap<u64, Vec<usize>>,
    event: Vec<Step>,
    pending: Option<crate::reduction::Event>,
    candidate: Option<Record>,
    initial: bool,
    cursor: usize,
    work: usize,
    cycle: bool,
    reached: bool,
}

impl Search {
    pub fn new(program: source::Program, target: source::Program) -> Result<Self, Failure> {
        if !target.rule.is_empty() {
            return Err(Failure::Declaration);
        }
        let compiled = Program::new(program.clone());
        let initial = Arc::new(State::initial(&compiled));
        let mut goal = compiled.clone();
        goal.initial = goal.input(&target.initial);
        let goal = State::initial(&goal);
        let signature = crate::fingerprint::state(&goal);
        let fingerprint = crate::fingerprint::state(&initial);
        let reached = initial.as_ref() == &goal;
        let compiled = Arc::new(compiled);
        Ok(Self {
            program,
            claim: target.initial,
            goal: Record::new(Arc::new(goal)),
            signature,
            runtime: crate::reduction::Search::new(compiled.clone(), initial.clone()),
            compiled,
            state: vec![Record::new(initial)],
            index: HashMap::from([(fingerprint, vec![0])]),
            event: Vec::new(),
            pending: None,
            candidate: None,
            initial: signature == fingerprint,
            cursor: 0,
            work: 0,
            cycle: false,
            reached,
        })
    }

    pub fn run(&mut self, budget: usize, limit: Limit) {
        for _ in 0..budget {
            let work = self.work;
            if self.reached || self.cycle {
                return;
            }
            let retained = self.state.len()
                + self.event.len()
                + self.runtime.record()
                + self
                    .pending
                    .as_ref()
                    .map_or(0, |event| event.fingerprint.retained() + 1)
                + usize::from(self.candidate.is_some());
            if retained >= limit.record {
                return;
            }
            if self.initial {
                if self.goal.canonical.get().is_none() {
                    self.goal.advance();
                    self.work += 1;
                    continue;
                }
                if self.state[0].canonical.get().is_none() {
                    self.state[0].advance();
                    self.work += 1;
                    continue;
                }
                self.reached = self.goal.canonical().state == self.state[0].canonical().state;
                self.initial = false;
                continue;
            }
            let event = if let Some(event) = self.pending.take() {
                event
            } else {
                let work = self.runtime.work;
                let event = self.runtime.run(limit);
                self.work += self.runtime.work - work;
                let Some(event) = event else {
                    if self.runtime.work == work {
                        return;
                    }
                    continue;
                };
                event
            };
            let fingerprint = event.fingerprint.value;
            let mut record = self
                .candidate
                .take()
                .unwrap_or_else(|| Record::new(event.state.clone()));
            let goal = self.signature == fingerprint && record.compatible(&self.goal);
            let comparison = goal
                || self.index.get(&fingerprint).is_some_and(|candidate| {
                    candidate
                        .iter()
                        .any(|&index| self.state[index].compatible(&record))
                });
            if comparison && self.work != work {
                self.pending = Some(event);
                self.candidate = Some(record);
                continue;
            }
            if comparison && record.canonical.get().is_none() {
                record.advance();
                self.work += 1;
                self.pending = Some(event);
                self.candidate = Some(record);
                continue;
            }
            if goal && self.goal.canonical.get().is_none() {
                self.goal.advance();
                self.work += 1;
                self.pending = Some(event);
                self.candidate = Some(record);
                continue;
            }
            if let Some(index) = self.index.get(&fingerprint).and_then(|candidate| {
                candidate.iter().copied().find(|&index| {
                    self.state[index].compatible(&record)
                        && self.state[index].canonical.get().is_none()
                })
            }) {
                self.state[index].advance();
                self.work += 1;
                self.pending = Some(event);
                self.candidate = Some(record);
                continue;
            }
            let known = self.index.get(&fingerprint).and_then(|candidate| {
                candidate.iter().copied().find(|&index| {
                    self.state[index].compatible(&record)
                        && self.state[index].canonical().state == record.canonical().state
                })
            });
            if known.is_none() && self.state.len() >= limit.state {
                self.pending = Some(event);
                self.candidate = Some(record);
                return;
            }
            let target = known.unwrap_or(self.state.len());
            self.reached = goal && record.canonical().state == self.goal.canonical().state;
            if known.is_none() {
                self.state.push(record);
                self.index.entry(fingerprint).or_default().push(target);
            }
            self.runtime
                .advance(event.state, &event.binding.world, event.fingerprint);
            self.event.push(Step {
                source: self.cursor,
                target,
                rule: event.rule,
                binding: event.binding,
                event: OnceLock::new(),
            });
            self.cursor = target;
            self.cycle = known.is_some();
        }
    }

    pub fn statistic(&self) -> Statistic {
        Statistic {
            refinement: self
                .state
                .iter()
                .filter(|record| record.signature.get().is_some())
                .count(),
            resolution: self
                .state
                .iter()
                .filter(|record| record.resolution.get().is_some())
                .count(),
            normalization: self
                .state
                .iter()
                .filter(|record| record.canonical.get().is_some())
                .count(),
            bucket: self.index.values().map(Vec::len).max().unwrap_or(0),
            preparation: self.runtime.preparation(),
            reuse: self.runtime.reuse(),
        }
    }

    pub fn inspect(&self, index: usize) -> Option<Node> {
        self.state.get(index).map(|state| {
            Node::new(
                index,
                &state.canonical().state,
                &self.compiled,
                Status::Supported,
            )
        })
    }

    pub fn transition(&self, index: usize) -> Option<&Event> {
        let step = self.event.get(index)?;
        Some(step.event.get_or_init(|| {
            let canonical = self.state[step.source].canonical();
            let place = |place: &Place| match *place {
                Place::World(world, token) => {
                    Place::World(canonical.world[world].unwrap(), canonical.resource[&token])
                }
                Place::Held(frame, token) => {
                    Place::Held(canonical.frame[frame].unwrap(), canonical.resource[&token])
                }
            };
            let selection = |value: &crate::basis::Set<Place>| {
                let mut value = value.iter().map(place).collect::<Vec<_>>();
                value.sort_unstable();
                value
            };
            Event {
                source: step.source,
                target: step.target,
                rule: self.compiled.rule[step.rule].name.clone(),
                footprint: selection(&step.binding.footprint),
                exact: selection(&step.binding.exact),
                read: selection(&step.binding.read),
            }
        }))
    }

    pub fn current(&self) -> Node {
        self.inspect(self.cursor).unwrap()
    }

    pub fn summary(&self) -> Summary {
        Summary {
            outcome: if self.reached {
                Outcome::Reached
            } else {
                Outcome::Unknown
            },
            witness: self.reached.then(|| self.current()),
            event: self.event.len(),
            work: self.work,
        }
    }

    pub fn report(&self) -> Report {
        Report {
            outcome: if self.reached {
                Outcome::Reached
            } else {
                Outcome::Unknown
            },
            witness: self.reached.then_some(self.cursor),
            work: self.work,
            program: self.program.clone(),
            target: self.claim.clone(),
            state: (0..self.state.len())
                .map(|index| self.inspect(index).unwrap())
                .collect(),
            event: (0..self.event.len())
                .map(|index| {
                    let event = self.transition(index).unwrap();
                    Event {
                        source: event.source,
                        target: event.target,
                        rule: event.rule.clone(),
                        footprint: event.footprint.clone(),
                        exact: event.exact.clone(),
                        read: event.read.clone(),
                    }
                })
                .collect(),
        }
    }
}

#[cfg(test)]
#[path = "test/collision.rs"]
mod test;
