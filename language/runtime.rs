mod application;
mod matching;
mod report;

use crate::flow::{Binding, Flow, Place};
use crate::program::Program;
use crate::slot::Slot;
use crate::source;
use crate::state::State;
use crate::support::{Atom, Clause, Support};
use crate::term::Term;
use indexmap::IndexSet;
use serde::Serialize;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::{Arc, OnceLock};

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Limit {
    pub state: usize,
    pub record: usize,
    pub world: usize,
    pub cell: usize,
    pub frame: usize,
}
impl Default for Limit {
    fn default() -> Self {
        Self {
            state: 80,
            record: 1_000_000,
            world: 4,
            cell: 12,
            frame: 10,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct View {
    source: usize,
    target: usize,
    flow: Flow,
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Application {
    view: usize,
    frame: usize,
    owner: Option<usize>,
    rule: usize,
    binding: Binding,
    capture: Option<usize>,
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Identity {
    source: usize,
    frame: usize,
    owner: Option<usize>,
    rule: usize,
    binding: Binding,
    environment: Option<Arc<State>>,
}
#[cfg(test)]
pub(crate) struct Transition<'a> {
    pub target: usize,
    pub rule: &'a str,
    pub binding: &'a Binding,
}

struct Event {
    identity: Identity,
    target: usize,
    flow: Flow,
    evidence: BTreeSet<usize>,
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Query {
    target: usize,
    frame: usize,
    pattern: Vec<Vec<Term>>,
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Consumer {
    view: usize,
    frame: usize,
    owner: Option<usize>,
    rule: usize,
    capture: Option<usize>,
    read: Option<Place>,
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Request {
    cache: usize,
    consumer: Consumer,
}
struct Cache {
    search: Option<crate::search::Search>,
    binding: Vec<Arc<Vec<Slot>>>,
    listener: Vec<usize>,
    retained: usize,
}
struct Normalization {
    identity: Identity,
    application: Vec<Application>,
    flow: Flow,
    search: Option<crate::canonical::Search>,
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
    pub(crate) state: IndexSet<Arc<State>>,
    index: Vec<Option<Arc<crate::index::Index>>>,
    indexed: usize,
    view: IndexSet<Arc<View>>,
    event: Vec<Event>,
    identity: HashMap<Identity, usize>,
    normalizing: HashMap<Identity, usize>,
    normalization: Vec<Option<Normalization>>,
    vacant: Vec<usize>,
    retained: usize,
    binding: usize,
    clause: IndexSet<Clause>,
    evaluation: OnceLock<Support>,
    matching: HashMap<Query, usize>,
    candidate: HashMap<(usize, usize), Arc<Vec<usize>>>,
    cache: Vec<Cache>,
    request: IndexSet<Request>,
    cursor: Vec<usize>,
    active: HashSet<usize>,
    outgoing: Vec<Vec<usize>>,
    incoming: Vec<Vec<usize>>,
    agenda: crate::agenda::Queue<Task>,
    pending: IndexSet<Application>,
    limit: Limit,
    pub(crate) work: usize,
    peak: usize,
    flying: usize,
}

impl Runtime {
    pub fn new(source: source::Program) -> Self {
        let program = Program::new(source);
        let initial = State::initial(&program);
        Self::seed(Arc::new(program), Arc::new(initial))
    }

    pub(crate) fn seed(program: Arc<Program>, initial: Arc<State>) -> Self {
        let mut runtime = Self {
            program,
            state: IndexSet::new(),
            index: Vec::new(),
            indexed: 0,
            view: IndexSet::new(),
            event: Vec::new(),
            identity: HashMap::new(),
            normalizing: HashMap::new(),
            normalization: Vec::new(),
            vacant: Vec::new(),
            retained: 0,
            binding: 0,
            clause: IndexSet::new(),
            evaluation: OnceLock::new(),
            matching: HashMap::new(),
            candidate: HashMap::new(),
            cache: Vec::new(),
            request: IndexSet::new(),
            cursor: Vec::new(),
            active: HashSet::new(),
            outgoing: Vec::new(),
            incoming: Vec::new(),
            agenda: crate::agenda::Queue::new(),
            pending: IndexSet::new(),
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
            rule: &self.program.rule[event.identity.rule].name,
            binding: &event.identity.binding,
        })
    }

    fn support(&mut self, head: Atom, premise: impl IntoIterator<Item = Atom>) {
        if self.clause.insert(Clause {
            head,
            premise: premise.into_iter().collect(),
        }) {
            self.evaluation.take();
        }
    }

    fn intern(&mut self, state: Arc<State>) -> usize {
        let (index, fresh) = self.state.insert_full(state);
        if !fresh {
            return index;
        }
        self.index.push(None);
        self.outgoing.push(Vec::new());
        self.incoming.push(Vec::new());
        let view = self.witness(View {
            source: index,
            target: index,
            flow: Flow::identity(&self.state[index]),
        });
        self.support(Atom::View(view), []);
        index
    }

    fn witness(&mut self, view: View) -> usize {
        let target = view.target;
        let (index, fresh) = self.view.insert_full(Arc::new(view));
        if !fresh {
            return index;
        }
        self.incoming[target].push(index);
        self.agenda.push_back(Task::Inspect(index));
        for &event in &self.outgoing[target] {
            self.agenda.defer(Task::Compose(index, event));
        }
        index
    }

    pub fn run(&mut self, budget: usize, limit: Option<Limit>) {
        self.execute(budget, limit, None);
    }

    pub fn parallel(
        &mut self,
        executor: &crate::executor::Executor,
        budget: usize,
        limit: Option<Limit>,
    ) {
        self.execute(budget, limit, Some(executor));
    }

    fn execute(
        &mut self,
        budget: usize,
        limit: Option<Limit>,
        executor: Option<&crate::executor::Executor>,
    ) {
        if let Some(limit) = limit {
            self.limit = limit;
            self.agenda.extend(self.pending.drain(..).map(Task::Apply));
        }
        let mut remaining = budget;
        while remaining > 0 && self.record() < self.limit.record {
            let mut batch = Vec::new();
            while batch.len() < remaining.min(32) {
                match self.agenda.front() {
                    Some(Task::Search(index)) => batch.push(crate::work::Work::Search(
                        *index,
                        self.cache[*index].search.take().unwrap(),
                    )),
                    Some(Task::Normalize(index)) => batch.push(crate::work::Work::Normalize(
                        *index,
                        self.normalization[*index]
                            .as_mut()
                            .unwrap()
                            .search
                            .take()
                            .unwrap(),
                    )),
                    _ => break,
                }
                self.agenda.pop_front();
            }
            if !batch.is_empty() {
                self.flying = batch.len();
                remaining -= batch.len();
                self.work += batch.len();
                let result = if let Some(executor) = executor {
                    executor.map(batch, crate::work::Work::advance)
                } else {
                    batch.into_iter().map(crate::work::Work::advance).collect()
                };
                for result in result {
                    self.flying -= 1;
                    match result {
                        crate::work::Result::Search(index, search, progress) => {
                            self.cache[index].search = Some(search);
                            self.search(index, progress);
                        }
                        crate::work::Result::Normalize(index, search, complete) => {
                            self.normalization[index].as_mut().unwrap().search = Some(search);
                            self.normalize(index, complete);
                        }
                    }
                    self.peak = self.peak.max(self.record());
                }
                continue;
            }
            let Some(task) = self.agenda.pop_front() else {
                break;
            };
            remaining -= 1;
            self.work += 1;
            match task {
                Task::Inspect(view) => self.inspect(view),
                Task::Deliver(request) => self.deliver(request),
                Task::Apply(application) => self.apply(application),
                Task::Compose(previous, event) => {
                    let source = self.view[previous].source;
                    let target = self.event[event].target;
                    let flow = self.view[previous].flow.compose(&self.event[event].flow);
                    let view = self.witness(View {
                        source,
                        target,
                        flow,
                    });
                    self.support(Atom::View(view), [Atom::View(previous), Atom::Event(event)]);
                }
                Task::Search(_) | Task::Normalize(_) => unreachable!(),
            }
            self.peak = self.peak.max(self.record());
        }
    }

    pub fn record(&self) -> usize {
        self.state.len()
            + self.index.len()
            + self.indexed
            + self.event.len()
            + self.view.len()
            + self.clause.len()
            + self.request.len()
            + self.cache.len()
            + self.candidate.len()
            + self.binding
            + self.retained
            + self.flying
            + self.normalization.len()
            + self.vacant.len()
            + self.agenda.len()
            + self.pending.len()
    }

    pub fn closed(&self) -> bool {
        self.agenda.is_empty() && self.pending.is_empty()
    }
}
