mod application;
mod composition;
mod environment;
mod matching;
mod normalization;
mod report;
mod table;

use crate::application::Owner;
use crate::flow::{Binding, Flow};
use crate::program::Program;
use crate::snapshot::Origin;
use crate::state::State;
use crate::support::{Atom, Clause};
use frontend::source;
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Limit {
    pub configuration: usize,
    pub coherence: usize,
    pub occurrence: usize,
    pub scope: usize,
    pub record: usize,
}

impl Default for Limit {
    fn default() -> Self {
        Self {
            configuration: 4_096,
            coherence: 64,
            occurrence: 256,
            scope: 64,
            record: 2_000_000,
        }
    }
}

impl Limit {
    pub(crate) fn admits(&self, coherence: usize, occurrence: usize, scope: usize) -> bool {
        coherence <= self.coherence && occurrence <= self.occurrence && scope <= self.scope
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct View {
    source: usize,
    target: usize,
    flow: Arc<Flow>,
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Application {
    view: usize,
    frame: usize,
    owner: Owner<usize>,
    rule: usize,
    binding: Binding,
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Identity {
    source: usize,
    frame: usize,
    owner: Owner<Arc<State>>,
    rule: usize,
    binding: Binding,
}
#[cfg(test)]
pub(crate) struct Transition<'transition> {
    pub target: usize,
    pub rule: usize,
    pub binding: &'transition Binding,
}

struct Event {
    identity: Identity,
    target: usize,
    flow: Arc<Flow>,
    evidence: BTreeSet<usize>,
}
#[derive(Clone)]
enum Task {
    Inspect(usize),
    Search(usize),
    Normalize(usize),
    Deliver(usize),
    Compose(usize, usize),
    Apply(Application),
}

pub struct Runtime {
    pub(crate) program: Arc<Program>,
    pub(crate) state: IndexSet<Arc<State>, hashing::Builder>,
    index: Vec<Option<Arc<crate::index::Index>>>,
    indexed: usize,
    view: IndexSet<Arc<View>, hashing::Builder>,
    origin: Vec<Option<Origin>>,
    event: Vec<Event>,
    normalization: normalization::Store,
    proof: crate::proof::Store,
    composition: crate::flow::Store,
    matching: table::Table,
    outgoing: Vec<Vec<usize>>,
    incoming: Vec<Vec<usize>>,
    agenda: crate::agenda::Queue<Task>,
    pending: IndexSet<Application, hashing::Builder>,
    limit: Limit,
    pub(crate) work: usize,
    peak: usize,
    flying: usize,
}

impl Runtime {
    pub fn new(source: &source::Program) -> Self {
        let program = Program::new(source);
        let initial = State::initial(&program);
        Self::seed(Arc::new(program), Arc::new(initial))
    }

    pub(crate) fn seed(program: Arc<Program>, initial: Arc<State>) -> Self {
        let mut runtime = Self {
            program,
            state: IndexSet::default(),
            index: Vec::new(),
            indexed: 0,
            view: IndexSet::default(),
            origin: Vec::new(),
            event: Vec::new(),
            normalization: normalization::Store::default(),
            proof: crate::proof::Store::default(),
            composition: crate::flow::Store::new(65_536),
            matching: table::Table::default(),
            outgoing: Vec::new(),
            incoming: Vec::new(),
            agenda: crate::agenda::Queue::new(),
            pending: IndexSet::default(),
            limit: Limit::default(),
            work: 0,
            peak: 0,
            flying: 0,
        };
        runtime.intern(initial);
        runtime.support(Atom::State(0), []);
        runtime.peak = runtime.record();
        runtime
    }

    #[cfg(test)]
    pub(crate) fn first(&self) -> Option<Transition<'_>> {
        self.event.first().map(|event| Transition {
            target: event.target,
            rule: event.identity.rule,
            binding: &event.identity.binding,
        })
    }

    fn support(&mut self, head: Atom, premise: impl IntoIterator<Item = Atom>) {
        self.proof.insert(Clause::new(head, premise));
    }

    fn intern(&mut self, state: Arc<State>) -> usize {
        let (index, fresh) = self.state.insert_full(state);
        if !fresh {
            return index;
        }
        self.index.push(None);
        self.outgoing.push(Vec::new());
        self.incoming.push(Vec::new());
        let view = self.witness(
            View {
                source: index,
                target: index,
                flow: Arc::new(Flow::identity(&self.state[index])),
            },
            None,
        );
        self.support(Atom::View(view), []);
        index
    }

    fn witness(&mut self, view: View, origin: Option<Origin>) -> usize {
        let target = view.target;
        let (index, fresh) = self.view.insert_full(Arc::new(view));
        if !fresh {
            return index;
        }
        self.origin.push(origin);
        self.incoming[target].push(index);
        self.agenda.push(Task::Inspect(index));
        for &event in &self.outgoing[target] {
            self.agenda.defer(Task::Compose(index, event));
        }
        index
    }

    pub fn run(&mut self, budget: usize, limit: Limit) {
        self.execute(budget, limit, None);
    }

    pub fn parallel(&mut self, executor: &crate::executor::Executor, budget: usize, limit: Limit) {
        self.execute(budget, limit, Some(executor));
    }

    fn execute(
        &mut self,
        budget: usize,
        limit: Limit,
        executor: Option<&crate::executor::Executor>,
    ) {
        if limit != self.limit {
            self.limit = limit;
            self.agenda.extend(self.pending.drain(..).map(Task::Apply));
        }
        let mut remaining = budget;
        while remaining > 0 {
            if self.record() >= self.limit.record {
                self.matching.evict();
                self.composition.evict();
                if self.record() >= self.limit.record {
                    break;
                }
            }
            let mut batch = Vec::new();
            while batch.len() < remaining.min(32) {
                match self.agenda.front() {
                    Some(Task::Search(index)) => batch.push(crate::work::Work::Search(
                        *index,
                        self.matching.take(*index),
                    )),
                    Some(Task::Normalize(index)) => batch.push(crate::work::Work::Normalize(
                        *index,
                        self.normalization.take(*index),
                    )),
                    _ => break,
                }
                self.agenda.pop();
            }
            if !batch.is_empty() {
                self.flying = batch.len();
                remaining -= batch.len();
                self.work += batch.len();
                let result = if let Some(executor) = executor
                    .filter(|executor| executor.concurrent() && crate::work::Work::parallel(&batch))
                {
                    executor.map(batch, crate::work::Work::advance)
                } else {
                    batch.into_iter().map(crate::work::Work::advance).collect()
                };
                for result in result {
                    self.flying -= 1;
                    match result {
                        crate::work::Progress::Search(index, search, progress) => {
                            self.search(index, search, progress);
                        }
                        crate::work::Progress::Normalize(index, search, complete) => {
                            self.normalize(index, search, complete);
                        }
                    }
                    self.peak = self.peak.max(self.record());
                }
                continue;
            }
            let Some(task) = self.agenda.pop() else {
                break;
            };
            remaining -= 1;
            self.work += 1;
            match task {
                Task::Inspect(view) => self.inspect(view),
                Task::Deliver(request) => self.deliver(request),
                Task::Apply(application) => self.apply(application),
                Task::Compose(previous, event) => self.compose(previous, event),
                Task::Search(_) | Task::Normalize(_) => unreachable!(),
            }
            self.peak = self.peak.max(self.record());
        }
    }

    pub(crate) fn record(&self) -> usize {
        self.state.len()
            + self.index.len()
            + self.indexed
            + self.event.len()
            + self.view.len()
            + self.proof.retained()
            + self.composition.retained()
            + self.matching.retained()
            + self.flying
            + self.normalization.retained()
            + self.agenda.len()
            + self.pending.len()
    }

    pub fn closed(&self) -> bool {
        self.agenda.is_empty() && self.pending.is_empty()
    }
}
