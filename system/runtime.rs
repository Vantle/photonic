use crate::flow::{self, Binding, Closure, Flow, Place};
use crate::matching::{self, Slot, Term};
use crate::program::{Program, Symbol};
use crate::source;
use crate::state::State;
use crate::support::{Atom, Clause, Support};
use indexmap::IndexSet;
use serde::Serialize;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::{Arc, OnceLock};
use std::task::Poll;

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
    environment: Option<State>,
}
struct Event {
    identity: Identity,
    target: usize,
    flow: Flow,
    evidence: BTreeSet<usize>,
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Match {
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
    result: flow::Applied,
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
    matching: HashMap<Match, usize>,
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
        Self::seed(Arc::new(program), initial)
    }

    pub(crate) fn seed(program: Arc<Program>, initial: State) -> Self {
        let mut runtime = Self {
            program,
            state: IndexSet::new(),
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

    pub(crate) fn first(&self) -> Option<usize> {
        self.event.first().map(|event| event.target)
    }

    fn support(&mut self, head: Atom, premise: impl IntoIterator<Item = Atom>) {
        self.clause.insert(Clause {
            head,
            premise: premise.into_iter().collect(),
        });
    }

    fn intern(&mut self, state: State) -> usize {
        let (index, fresh) = self.state.insert_full(Arc::new(state));
        if !fresh {
            return index;
        }
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

    fn wake(&mut self, request: usize) {
        if self.active.insert(request) {
            self.agenda.push_back(Task::Deliver(request));
        }
    }

    fn matching(
        &mut self,
        target: usize,
        frame: usize,
        pattern: Vec<Vec<Term>>,
        consumer: Consumer,
    ) {
        let key = Match {
            target,
            frame,
            pattern,
        };
        let cache = if let Some(&cache) = self.matching.get(&key) {
            cache
        } else {
            let cache = self.cache.len();
            self.cache.push(Cache {
                search: Some(crate::search::Search::new(
                    key.pattern.clone(),
                    self.state[target].clone(),
                    frame,
                )),
                binding: Vec::new(),
                listener: Vec::new(),
                retained: 0,
            });
            self.matching.insert(key, cache);
            self.agenda.push_back(Task::Search(cache));
            cache
        };
        let (index, fresh) = self.request.insert_full(Request { cache, consumer });
        if !fresh {
            return;
        }
        self.cursor.push(0);
        self.cache[cache].listener.push(index);
        if !self.cache[cache].binding.is_empty() {
            self.wake(index);
        }
    }

    fn search(&mut self, cache: usize, progress: Poll<Option<Vec<Slot>>>) {
        let retained = self.cache[cache].search.as_ref().unwrap().retained();
        self.retained = self.retained - self.cache[cache].retained + retained;
        self.cache[cache].retained = retained;
        match progress {
            Poll::Ready(None) => {
                self.retained -= self.cache[cache].retained;
                self.cache[cache].retained = 0;
                self.cache[cache].search = None;
                return;
            }
            Poll::Ready(Some(binding)) => {
                self.binding += 1;
                self.cache[cache].binding.push(Arc::new(binding));
                for index in self.cache[cache].listener.clone() {
                    self.wake(index);
                }
            }
            Poll::Pending => {}
        }
        self.agenda.push_back(Task::Search(cache));
    }

    fn deliver(&mut self, index: usize) {
        self.active.remove(&index);
        let request = self.request[index].clone();
        let cache = &self.cache[request.cache];
        let Some(selection) = cache.binding.get(self.cursor[index]).cloned() else {
            return;
        };
        self.cursor[index] += 1;
        if self.cursor[index] < cache.binding.len() {
            self.wake(index);
        }
        let consumer = request.consumer;
        let view = self.view[consumer.view].clone();
        if let Some(Place::World(site, _)) = consumer.read {
            if !selection.iter().any(|slot| slot.world == site) {
                return;
            }
        }
        let selected = selection
            .iter()
            .map(|slot| (slot.world, slot.token.clone()))
            .collect::<Vec<_>>();
        let Some(mut binding) = view.flow.project(
            &self.state[view.source],
            &self.state[view.target],
            &selected,
            consumer.frame,
        ) else {
            return;
        };
        if let Some(read) = consumer.read {
            binding.read = view.flow.resource[&read].clone();
        }
        self.agenda.push_back(Task::Apply(Application {
            view: consumer.view,
            frame: consumer.frame,
            owner: consumer.owner,
            rule: consumer.rule,
            binding,
            capture: consumer.capture,
        }));
    }

    fn inspect(&mut self, index: usize) {
        let view = self.view[index].clone();
        let source = self.state[view.source].clone();
        let target = self.state[view.target].clone();
        let available = target
            .world
            .iter()
            .flat_map(|world| world.particle.iter().map(|token| token.value))
            .collect::<HashSet<_>>();
        for frame in 0..source.frame.len() {
            if frame != 0 && !source.world.iter().any(|world| world.frame == frame) {
                continue;
            }
            let mut owner = Some(frame);
            while let Some(current) = owner {
                let rule = self.program.scope[source.frame[current].scope].candidate(&available);
                for rule in rule {
                    if self.program.rule[rule]
                        .input
                        .iter()
                        .flatten()
                        .any(|symbol| !available.contains(symbol))
                    {
                        continue;
                    }
                    let input = if self.program.rule[rule].input.is_empty() {
                        vec![Vec::new()]
                    } else {
                        self.program.rule[rule].input.clone()
                    };
                    let capture = view
                        .flow
                        .frame
                        .iter()
                        .position(|&value| value == Some(current));
                    let pattern = matching::pattern(&input, capture);
                    for destination in 0..target.frame.len() {
                        if view.flow.frame[destination] != Some(frame) {
                            continue;
                        }
                        self.matching(
                            view.target,
                            destination,
                            pattern.clone(),
                            Consumer {
                                view: index,
                                frame,
                                owner: Some(current),
                                rule,
                                capture: None,
                                read: None,
                            },
                        );
                    }
                }
                owner = source.frame[current].lexical;
            }
        }
        for (site, world) in target.world.iter().enumerate() {
            let Some(frame) = view.flow.frame[world.frame] else {
                continue;
            };
            for token in &world.particle {
                let Symbol::Rule(rule) = token.value else {
                    continue;
                };
                if self.program.rule[rule]
                    .input
                    .iter()
                    .flatten()
                    .any(|symbol| !available.contains(symbol))
                {
                    continue;
                }
                let input = if self.program.rule[rule].input.is_empty() {
                    vec![Vec::new()]
                } else {
                    self.program.rule[rule].input.clone()
                };
                let pattern = matching::pattern(&input, token.capture);
                self.matching(
                    view.target,
                    world.frame,
                    pattern,
                    Consumer {
                        view: index,
                        frame,
                        owner: token.capture.and_then(|capture| view.flow.frame[capture]),
                        rule,
                        capture: token.capture,
                        read: Some(Place::World(site, token.id)),
                    },
                );
            }
        }
    }

    fn apply(&mut self, application: Application) {
        let view = self.view[application.view].clone();
        let environment = application
            .capture
            .map(|capture| self.state[view.target].environment(capture));
        let key = Identity {
            source: view.source,
            frame: application.frame,
            owner: application.owner,
            rule: application.rule,
            binding: application.binding.clone(),
            environment: environment.clone(),
        };
        if let Some(&event) = self.identity.get(&key) {
            self.justify(event, application);
            return;
        }
        if let Some(&index) = self.normalizing.get(&key) {
            self.normalization[index]
                .as_mut()
                .unwrap()
                .application
                .push(application);
            return;
        }
        let closure = application.capture.map(|capture| Closure {
            state: &self.state[view.target],
            flow: &view.flow,
            capture,
        });
        let result = flow::apply(
            &self.state[view.source],
            application.frame,
            application.owner,
            &self.program.rule[application.rule],
            &application.binding,
            closure,
        );
        if result.state.world.len() > self.limit.world
            || result.state.cells() > self.limit.cell
            || result.state.reachable().len() > self.limit.frame
        {
            self.pending.insert(application);
            return;
        }
        let search = crate::canonical::Search::new(Arc::new(result.state.clone()));
        let index = self.vacant.pop().unwrap_or_else(|| {
            let index = self.normalization.len();
            self.normalization.push(None);
            index
        });
        self.normalizing.insert(key.clone(), index);
        self.normalization[index] = Some(Normalization {
            identity: key,
            application: vec![application],
            result,
            search: Some(search),
        });
        self.agenda.push_back(Task::Normalize(index));
    }

    fn normalize(&mut self, index: usize, complete: bool) {
        if !complete {
            self.agenda.push_back(Task::Normalize(index));
            return;
        }
        let normalization = self.normalization[index].take().unwrap();
        self.vacant.push(index);
        self.normalizing.remove(&normalization.identity);
        let result = normalization
            .result
            .rename(normalization.search.unwrap().finish().unwrap());
        if self.state.len() >= self.limit.state && !self.state.contains(&result.state) {
            self.pending.extend(normalization.application);
            return;
        }
        let source = normalization.identity.source;
        let target = self.intern(result.state);
        let event = self.event.len();
        self.identity.insert(normalization.identity.clone(), event);
        self.event.push(Event {
            identity: normalization.identity,
            target,
            flow: result.flow,
            evidence: BTreeSet::new(),
        });
        self.outgoing[source].push(event);
        for &previous in &self.incoming[source] {
            self.agenda.defer(Task::Compose(previous, event));
        }
        self.support(Atom::State(target), [Atom::Event(event)]);
        for application in normalization.application {
            self.justify(event, application);
        }
    }

    fn justify(&mut self, event: usize, application: Application) {
        let source = self.event[event].identity.source;
        self.event[event].evidence.insert(application.view);
        self.support(
            Atom::Event(event),
            [Atom::State(source), Atom::View(application.view)],
        );
    }

    pub fn run(&mut self, steps: usize, limit: Option<Limit>) {
        self.execute(steps, limit, None);
    }

    pub fn parallel(
        &mut self,
        executor: &crate::executor::Executor,
        steps: usize,
        limit: Option<Limit>,
    ) {
        self.execute(steps, limit, Some(executor));
    }

    fn execute(
        &mut self,
        steps: usize,
        limit: Option<Limit>,
        executor: Option<&crate::executor::Executor>,
    ) {
        self.evaluation.take();
        if let Some(limit) = limit {
            self.limit = limit;
            self.agenda.extend(self.pending.drain(..).map(Task::Apply));
        }
        let mut remaining = steps;
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
            + self.event.len()
            + self.view.len()
            + self.clause.len()
            + self.request.len()
            + self.cache.len()
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

    pub fn snapshot(&self) -> crate::snapshot::Snapshot {
        let support = self
            .evaluation
            .get_or_init(|| Support::new(self.clause.iter().cloned()));
        crate::snapshot::Snapshot {
            closed: self.closed(),
            record: self.record(),
            peak: self.peak,
            queued: self.agenda.len(),
            deferred: self.pending.len(),
            work: self.work,
            limit: self.limit,
            state: self
                .state
                .iter()
                .enumerate()
                .map(|(id, state)| {
                    crate::snapshot::Node::new(
                        id,
                        state,
                        &self.program,
                        support.status(Atom::State(id)),
                    )
                })
                .collect(),
            event: self
                .event
                .iter()
                .enumerate()
                .map(|(id, event)| crate::snapshot::Event {
                    id,
                    source: event.identity.source,
                    target: event.target,
                    rule: self.program.rule[event.identity.rule].name.clone(),
                    status: support.status(Atom::Event(id)),
                    footprint: event.identity.binding.footprint.iter().copied().collect(),
                    exact: event.identity.binding.exact.iter().copied().collect(),
                    read: event.identity.binding.read.iter().copied().collect(),
                    evidence: event.evidence.iter().copied().collect(),
                })
                .collect(),
            view: self
                .view
                .iter()
                .enumerate()
                .map(|(id, view)| crate::snapshot::View {
                    id,
                    source: view.source,
                    target: view.target,
                    status: support.status(Atom::View(id)),
                    resource: view
                        .flow
                        .resource
                        .iter()
                        .map(|(&target, source)| crate::snapshot::Link {
                            target,
                            source: source.iter().copied().collect(),
                        })
                        .collect(),
                    context: view
                        .flow
                        .context
                        .iter()
                        .map(|value| value.iter().copied().collect())
                        .collect(),
                    frame: view.flow.frame.clone(),
                })
                .collect(),
        }
    }
}
