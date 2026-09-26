mod report;

use crate::place::Place;
use crate::prism::Outcome;
use crate::program::Program;
use crate::runtime::Limit;
use crate::snapshot::Node;
use crate::state::{Canonical, State};
use crate::status::Status;
use frontend::source;
use hashing::Builder;
use serde::Serialize;
use smallvec::{SmallVec, smallvec};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};
use std::task::Poll;

#[derive(Clone, Debug, Serialize)]
pub struct Event {
    pub source: usize,
    pub target: usize,
    pub rule: usize,
    pub footprint: Vec<Place>,
    pub exact: Vec<Place>,
    pub read: Vec<Place>,
}

#[derive(Debug, Serialize)]
pub struct Report<
    Configuration = Vec<Node>,
    Transition = Vec<Event>,
    Source = source::Program,
    Target = Option<source::Program>,
> {
    pub definition: Vec<crate::snapshot::Definition>,
    pub outcome: Outcome,
    pub witness: Option<usize>,
    pub work: usize,
    pub program: Source,
    pub target: Target,
    pub state: Configuration,
    pub event: Transition,
}

pub struct Summary {
    pub outcome: Outcome,
    pub witness: Option<Node>,
    pub length: usize,
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

struct Pending {
    event: crate::reduction::Event,
    record: Record,
}

struct Goal {
    record: Record,
    signature: u64,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Stage {
    Initial,
    Walk,
    Reached,
    Cycle,
}

pub struct Search {
    program: source::Program,
    target: Option<source::Program>,
    compiled: Arc<Program>,
    goal: Option<Goal>,
    runtime: crate::reduction::Search,
    structure: crate::structure::Structure,
    state: Vec<Record>,
    index: HashMap<u64, SmallVec<[usize; 1]>, Builder>,
    event: Vec<Step>,
    pending: Option<Pending>,
    stage: Stage,
    cursor: usize,
    work: usize,
}

impl Search {
    pub fn new(program: source::Program, target: Option<source::Program>) -> Self {
        let compiled = Program::new(&program);
        let initial = Arc::new(State::initial(&compiled));
        let fingerprint = crate::fingerprint::state(&initial);
        let goal = target
            .as_ref()
            .and_then(|target| State::target(&compiled, target))
            .map(|state| Goal {
                signature: crate::fingerprint::state(&state),
                record: Record::new(Arc::new(state)),
            });
        let stage = match &goal {
            Some(goal) if goal.record.state == initial => Stage::Reached,
            Some(goal) if goal.signature == fingerprint => Stage::Initial,
            _ => Stage::Walk,
        };
        let compiled = Arc::new(compiled);
        Self {
            program,
            target,
            goal,
            runtime: crate::reduction::Search::new(compiled.clone(), initial.clone()),
            structure: crate::structure::Structure::default(),
            compiled,
            state: vec![Record::new(initial)],
            index: HashMap::from_iter([(fingerprint, smallvec![0])]),
            event: Vec::new(),
            pending: None,
            stage,
            cursor: 0,
            work: 0,
        }
    }

    pub fn run(&mut self, budget: usize, limit: Limit) {
        let mut remaining = budget;
        while remaining > 0 {
            remaining -= 1;
            let start = self.work;
            if matches!(self.stage, Stage::Reached | Stage::Cycle) || !self.fits(limit.record) {
                return;
            }
            if self.stage == Stage::Initial {
                self.initialize();
                continue;
            }
            let (event, mut record) = if let Some(Pending { event, record }) = self.pending.take() {
                (event, record)
            } else {
                let skipped = self.runtime.skip(remaining + 1);
                if skipped > 0 {
                    self.work += skipped;
                    remaining -= skipped - 1;
                    continue;
                }
                let before = self.runtime.work();
                let event = self.runtime.run(limit);
                self.work += self.runtime.work() - before;
                let event = match event {
                    Poll::Ready(Some(event)) => event,
                    Poll::Ready(None) => return,
                    Poll::Pending => continue,
                };
                let record = Record::new(event.state.clone());
                (event, record)
            };
            let fingerprint = event.fingerprint.value();
            let aimed = self
                .goal
                .as_ref()
                .is_some_and(|goal| goal.signature == fingerprint);
            if record.signature.get().is_none() && (aimed || self.index.contains_key(&fingerprint))
            {
                let signature = self.structure.advance(&record.state);
                record.signature.set(signature).unwrap();
            }
            let goal = aimed
                && self
                    .goal
                    .as_ref()
                    .is_some_and(|goal| record.compatible(&goal.record));
            if goal || self.comparable(&record, fingerprint) {
                let worked = self.work != start;
                if worked || self.normalize(&mut record, goal, fingerprint) {
                    self.work += usize::from(!worked);
                    self.pending = Some(Pending { event, record });
                    continue;
                }
            }
            let known = self.known(&record, fingerprint);
            if known.is_none() && self.state.len() >= limit.configuration {
                self.pending = Some(Pending { event, record });
                return;
            }
            let target = known.unwrap_or(self.state.len());
            let reached = goal
                && self
                    .goal
                    .as_ref()
                    .is_some_and(|goal| record.canonical().state == goal.record.canonical().state);
            if known.is_none() {
                self.state.push(record);
                self.index.entry(fingerprint).or_default().push(target);
            }
            self.runtime
                .advance(event.state, &event.change, event.fingerprint, event.layout);
            self.event.push(Step {
                source: self.cursor,
                target,
                rule: event.rule,
                binding: event.binding,
                event: OnceLock::new(),
            });
            self.cursor = target;
            self.stage = if reached {
                Stage::Reached
            } else if known.is_some() {
                Stage::Cycle
            } else {
                Stage::Walk
            };
        }
    }

    fn retained(&self) -> usize {
        self.state.len()
            + self.event.len()
            + self.runtime.record()
            + self.structure.retained()
            + self
                .pending
                .as_ref()
                .map_or(0, |pending| pending.event.retained() + 2)
    }

    fn fits(&mut self, record: usize) -> bool {
        let mut retained = self.retained();
        if retained < record {
            return true;
        }
        retained -= self.runtime.evict();
        retained -= self
            .pending
            .as_mut()
            .map_or(0, |pending| pending.event.evict());
        retained -= self.structure.retained();
        self.structure = crate::structure::Structure::default();
        retained < record
    }

    fn initialize(&mut self) {
        let Some(goal) = &mut self.goal else {
            self.stage = Stage::Walk;
            return;
        };
        if goal.record.canonical.get().is_none() {
            goal.record.advance();
            self.work += 1;
            return;
        }
        if self.state[0].canonical.get().is_none() {
            self.state[0].advance();
            self.work += 1;
            return;
        }
        self.stage = if goal.record.canonical().state == self.state[0].canonical().state {
            Stage::Reached
        } else {
            Stage::Walk
        };
    }

    fn comparable(&self, record: &Record, fingerprint: u64) -> bool {
        self.index.get(&fingerprint).is_some_and(|candidate| {
            candidate
                .iter()
                .any(|&index| self.state[index].compatible(record))
        })
    }

    fn known(&self, record: &Record, fingerprint: u64) -> Option<usize> {
        self.index.get(&fingerprint).and_then(|candidate| {
            candidate.iter().copied().find(|&index| {
                self.state[index].compatible(record)
                    && self.state[index].canonical().state == record.canonical().state
            })
        })
    }

    fn reached(&self) -> bool {
        self.stage == Stage::Reached
    }

    fn normalize(&mut self, record: &mut Record, goal: bool, fingerprint: u64) -> bool {
        if record.canonical.get().is_none() {
            record.advance();
            return true;
        }
        if goal
            && let Some(goal) = &mut self.goal
            && goal.record.canonical.get().is_none()
        {
            goal.record.advance();
            return true;
        }
        let Some(index) = self.index.get(&fingerprint).and_then(|candidate| {
            candidate.iter().copied().find(|&index| {
                self.state[index].compatible(record) && self.state[index].canonical.get().is_none()
            })
        }) else {
            return false;
        };
        self.state[index].advance();
        true
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
            bucket: self.index.values().map(SmallVec::len).max().unwrap_or(0),
            preparation: self.runtime.preparation(),
            reuse: self.runtime.reuse(),
        }
    }

    pub fn inspect(&self, index: usize) -> Option<Node> {
        self.state.get(index).map(|state| {
            crate::render::Builder::new(&self.compiled).node(
                index,
                &state.canonical().state,
                Status::Supported,
            )
        })
    }

    pub fn transition(&self, index: usize) -> Option<&Event> {
        let step = self.event.get(index)?;
        Some(step.event.get_or_init(|| {
            let canonical = self.state[step.source].canonical();
            let place = |place: &Place| canonical.place(*place).unwrap();
            let selection = |value: &crate::basis::Set<Place>| {
                let mut value = value.iter().map(place).collect::<Vec<_>>();
                value.sort_unstable();
                value
            };
            Event {
                source: step.source,
                target: step.target,
                rule: step.rule,
                footprint: selection(&step.binding.footprint),
                exact: selection(&step.binding.exact),
                read: selection(&step.binding.read),
            }
        }))
    }

    pub fn current(&self) -> Node {
        self.inspect(self.cursor).unwrap()
    }

    fn outcome(&self) -> Outcome {
        if self.reached() {
            Outcome::Reached
        } else {
            Outcome::Unknown
        }
    }

    pub fn summary(&self) -> Summary {
        Summary {
            outcome: self.outcome(),
            witness: self.reached().then(|| self.current()),
            length: self.event.len(),
            work: self.work,
        }
    }
}

#[cfg(test)]
#[path = "test/collision.rs"]
mod test;
