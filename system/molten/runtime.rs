use crate::flow::{self, Binding, Closure, Flow, Place};
use crate::matching::{self, Capture, Slot, Term};
use crate::program::{Program, Symbol};
use crate::source;
use crate::state::State;
use crate::support::{Atom, Clause, Support};
use indexmap::IndexSet;
use serde::Serialize;
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Limit {
    pub state: usize,
    pub world: usize,
    pub cell: usize,
    pub frame: usize,
}
impl Default for Limit {
    fn default() -> Self {
        Self {
            state: 80,
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
struct Query {
    source: usize,
    frame: usize,
    pattern: Vec<Vec<Term>>,
    closed: bool,
}
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Match {
    target: usize,
    frame: usize,
    pattern: Vec<Vec<Term>>,
}
#[derive(Clone)]
enum Task {
    Inspect(usize),
    Compose(usize, usize),
    Apply(Application),
    Answer(usize, usize),
}

pub struct Runtime {
    pub(crate) program: Program,
    pub(crate) state: IndexSet<Arc<State>>,
    view: IndexSet<Arc<View>>,
    event: Vec<Event>,
    identity: HashMap<Identity, usize>,
    query: IndexSet<Query>,
    clause: IndexSet<Clause>,
    cache: HashMap<Match, Arc<Vec<Vec<Slot>>>>,
    outgoing: Vec<Vec<usize>>,
    incoming: Vec<Vec<usize>>,
    origin: Vec<Vec<usize>>,
    subscription: Vec<Vec<usize>>,
    agenda: VecDeque<Task>,
    pending: IndexSet<Application>,
    limit: Limit,
    work: usize,
}

impl Runtime {
    pub fn new(source: source::Program) -> Self {
        let program = Program::new(source);
        let initial = State::initial(&program);
        let mut runtime = Self {
            program,
            state: IndexSet::new(),
            view: IndexSet::new(),
            event: Vec::new(),
            identity: HashMap::new(),
            query: IndexSet::new(),
            clause: IndexSet::new(),
            cache: HashMap::new(),
            outgoing: Vec::new(),
            incoming: Vec::new(),
            origin: Vec::new(),
            subscription: Vec::new(),
            agenda: VecDeque::new(),
            pending: IndexSet::new(),
            limit: Limit::default(),
            work: 0,
        };
        runtime.intern(initial);
        runtime.support(Atom::State(0), [], []);
        runtime
    }

    fn support(
        &mut self,
        head: Atom,
        positive: impl IntoIterator<Item = Atom>,
        negative: impl IntoIterator<Item = Atom>,
    ) {
        self.clause.insert(Clause {
            head,
            positive: positive.into_iter().collect(),
            negative: negative.into_iter().collect(),
        });
    }

    fn intern(&mut self, state: State) -> usize {
        let (index, fresh) = self.state.insert_full(Arc::new(state));
        if !fresh {
            return index;
        }
        self.outgoing.push(Vec::new());
        self.incoming.push(Vec::new());
        self.origin.push(Vec::new());
        self.subscription.push(Vec::new());
        let view = self.witness(View {
            source: index,
            target: index,
            flow: Flow::identity(&self.state[index]),
        });
        self.support(Atom::View(view), [], []);
        index
    }

    fn witness(&mut self, view: View) -> usize {
        let source = view.source;
        let target = view.target;
        let (index, fresh) = self.view.insert_full(Arc::new(view));
        if !fresh {
            return index;
        }
        self.incoming[target].push(index);
        self.origin[source].push(index);
        self.agenda.push_back(Task::Inspect(index));
        for &event in &self.outgoing[target] {
            self.agenda.push_back(Task::Compose(index, event));
        }
        index
    }

    fn matching(
        &mut self,
        target: usize,
        frame: usize,
        pattern: Vec<Vec<Term>>,
    ) -> Arc<Vec<Vec<Slot>>> {
        let key = Match {
            target,
            frame,
            pattern,
        };
        self.cache
            .entry(key.clone())
            .or_insert_with(|| Arc::new(matching::world(&key.pattern, &self.state[target], frame)))
            .clone()
    }

    fn inspect(&mut self, index: usize) {
        let view = self.view[index].clone();
        let source = self.state[view.source].clone();
        let target = self.state[view.target].clone();
        for frame in 0..source.frame.len() {
            if frame != 0 && !source.world.iter().any(|world| world.frame == frame) {
                continue;
            }
            let mut owner = Some(frame);
            while let Some(current) = owner {
                let rule = self.program.scope[source.frame[current].scope].rule.clone();
                for rule in rule {
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
                    let pattern = matching::pattern(&input, capture, None);
                    for destination in 0..target.frame.len() {
                        if view.flow.frame[destination] != Some(frame) {
                            continue;
                        }
                        let selection = self.matching(view.target, destination, pattern.clone());
                        for selection in selection.iter() {
                            let selected = selection
                                .iter()
                                .map(|slot| (slot.world, slot.token.clone()))
                                .collect::<Vec<_>>();
                            if let Some(binding) =
                                view.flow.project(&source, &target, &selected, frame)
                            {
                                self.agenda.push_back(Task::Apply(Application {
                                    view: index,
                                    frame,
                                    owner: Some(current),
                                    rule,
                                    binding,
                                    capture: None,
                                }));
                            }
                        }
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
                let input = if self.program.rule[rule].input.is_empty() {
                    vec![Vec::new()]
                } else {
                    self.program.rule[rule].input.clone()
                };
                let pattern = matching::pattern(&input, token.capture, None);
                let selection = self.matching(view.target, world.frame, pattern);
                for selection in selection.iter() {
                    if !selection.iter().any(|slot| slot.world == site) {
                        continue;
                    }
                    let selected = selection
                        .iter()
                        .map(|slot| (slot.world, slot.token.clone()))
                        .collect::<Vec<_>>();
                    if let Some(mut binding) = view.flow.project(&source, &target, &selected, frame)
                    {
                        binding.read = view.flow.resource[&Place::World(site, token.id)].clone();
                        self.agenda.push_back(Task::Apply(Application {
                            view: index,
                            frame,
                            owner: token.capture.and_then(|capture| view.flow.frame[capture]),
                            rule,
                            binding,
                            capture: token.capture,
                        }));
                    }
                }
            }
        }
        for &query in &self.subscription[view.source] {
            self.agenda.push_back(Task::Answer(index, query));
        }
    }

    fn impossible(&self, source: usize, pattern: &[Vec<Term>]) -> bool {
        if !self.program.code.is_empty() {
            return false;
        }
        let state = &self.state[source];
        let mut possible = state
            .world
            .iter()
            .flat_map(|world| &world.particle)
            .chain(state.frame.iter().flat_map(|frame| &frame.held))
            .map(|token| token.value)
            .collect::<BTreeSet<_>>();
        loop {
            let before = possible.len();
            for rule in &self.program.rule {
                if rule
                    .input
                    .iter()
                    .flatten()
                    .all(|value| possible.contains(value))
                {
                    possible.extend(
                        rule.output
                            .iter()
                            .flat_map(|output| output.particle.iter().copied()),
                    );
                }
            }
            if possible.len() == before {
                break;
            }
        }
        pattern
            .iter()
            .flatten()
            .any(|term| !possible.contains(&term.value))
    }

    fn obligation(&mut self, source: usize, frame: usize, pattern: Vec<Vec<Term>>) -> usize {
        let closed = self.impossible(source, &pattern);
        let (index, fresh) = self.query.insert_full(Query {
            source,
            frame,
            pattern,
            closed,
        });
        if fresh {
            self.subscription[source].push(index);
            for &view in &self.origin[source] {
                self.agenda.push_back(Task::Answer(view, index));
            }
        }
        index
    }

    fn answer(&mut self, index: usize, query: usize) {
        let view = self.view[index].clone();
        let obligation = self.query[query].clone();
        let mut pattern = obligation.pattern;
        for term in pattern.iter_mut().flatten() {
            if let Some(Capture::Frame(capture)) = &mut term.capture {
                *capture = capture.and_then(|capture| {
                    view.flow
                        .frame
                        .iter()
                        .position(|&value| value == Some(capture))
                });
            }
        }
        for frame in 0..self.state[view.target].frame.len() {
            if view.flow.frame[frame] != Some(obligation.frame) {
                continue;
            }
            let selection = self.matching(view.target, frame, pattern.clone());
            for selection in selection.iter() {
                let selected = selection
                    .iter()
                    .map(|slot| (slot.world, slot.token.clone()))
                    .collect::<Vec<_>>();
                if view
                    .flow
                    .project(
                        &self.state[view.source],
                        &self.state[view.target],
                        &selected,
                        obligation.frame,
                    )
                    .is_some()
                {
                    self.support(Atom::Query(query), [Atom::View(index)], []);
                    return;
                }
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
        let event = if let Some(&event) = self.identity.get(&key) {
            event
        } else {
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
            let result = result.canonical();
            if self.state.len() >= self.limit.state && !self.state.contains(&result.state) {
                self.pending.insert(application);
                return;
            }
            let target = self.intern(result.state);
            let event = self.event.len();
            self.identity.insert(key.clone(), event);
            self.event.push(Event {
                identity: key,
                target,
                flow: result.flow,
                evidence: BTreeSet::new(),
            });
            self.outgoing[view.source].push(event);
            for &previous in &self.incoming[view.source] {
                self.agenda.push_back(Task::Compose(previous, event));
            }
            self.support(Atom::State(target), [Atom::Event(event)], []);
            event
        };
        self.event[event].evidence.insert(application.view);
        let negative = if let Some(pattern) = self.program.rule[application.rule].negative.clone() {
            let pattern = matching::pattern(
                &pattern,
                application.owner,
                if application.owner.is_none() {
                    environment.as_ref()
                } else {
                    None
                },
            );
            vec![Atom::Query(self.obligation(
                view.source,
                application.frame,
                pattern,
            ))]
        } else {
            Vec::new()
        };
        self.support(
            Atom::Event(event),
            [Atom::State(view.source), Atom::View(application.view)],
            negative,
        );
    }

    pub fn run(&mut self, steps: usize, limit: Option<Limit>) {
        if let Some(limit) = limit {
            self.limit = limit;
            self.agenda.extend(self.pending.drain(..).map(Task::Apply));
        }
        for _ in 0..steps {
            let Some(task) = self.agenda.pop_front() else {
                break;
            };
            self.work += 1;
            match task {
                Task::Inspect(view) => self.inspect(view),
                Task::Apply(application) => self.apply(application),
                Task::Answer(view, query) => self.answer(view, query),
                Task::Compose(previous, event) => {
                    let source = self.view[previous].source;
                    let target = self.event[event].target;
                    let flow = self.view[previous].flow.compose(&self.event[event].flow);
                    let view = self.witness(View {
                        source,
                        target,
                        flow,
                    });
                    self.support(
                        Atom::View(view),
                        [Atom::View(previous), Atom::Event(event)],
                        [],
                    );
                }
            }
        }
    }

    pub fn closed(&self) -> bool {
        self.agenda.is_empty() && self.pending.is_empty()
    }

    pub fn snapshot(&self) -> crate::snapshot::Snapshot {
        let support = Support::new(
            self.clause.iter().cloned(),
            self.query
                .iter()
                .enumerate()
                .filter_map(|(index, query)| (!self.closed() && !query.closed).then_some(index)),
        );
        crate::snapshot::Snapshot {
            closed: self.closed(),
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
            query: self
                .query
                .iter()
                .enumerate()
                .map(|(id, query)| crate::snapshot::Query {
                    id,
                    source: query.source,
                    frame: query.frame,
                    status: support.status(Atom::Query(id)),
                })
                .collect(),
        }
    }
}
