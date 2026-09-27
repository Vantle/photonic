mod agreement;
mod capture;
mod direct;
mod passage;
mod pool;
mod scan;
mod space;
mod trace;

use crate::application::Owner;
use crate::catalog::Catalog;
use crate::executor::Executor;
use crate::flow::{Binding, Closure};
use crate::program::Program;
use crate::runtime::Limit;
use crate::state::{Canonical, State};
use capture::{Capture, Environment};
use hashing::Builder;
use indexmap::{IndexMap, IndexSet};
use passage::Passage;
use pool::{Demand, Pool};
use serde::Serialize;
use space::{Found, Space};
use std::collections::{BTreeMap, HashMap};
use std::num::NonZeroU32;
use std::ops::Range;
use std::sync::{Arc, Mutex};
use trace::Trace;

const CHUNK: usize = 256;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Identity {
    source: usize,
    frame: usize,
    owner: Owner<Arc<Environment>>,
    rule: usize,
    binding: Binding,
}

struct Event {
    source: usize,
    slot: usize,
    target: usize,
    direct: bool,
}

// The traces of a configuration before the first count have crossed its incoming events before
// the second, so each trace crosses each event once.
#[derive(Clone, Copy, Default)]
struct Progress {
    trace: usize,
    event: usize,
}

struct Crossing {
    event: usize,
    range: Range<usize>,
}

// Where each trace of a crossing lands: nowhere when its frame did not exist before the event,
// otherwise at a trace the source already had or at one of the new traces the crossing carried.
#[derive(Clone, Copy)]
enum Landing {
    Lost,
    Known(usize),
    New(usize),
}

struct Carried {
    event: usize,
    source: usize,
    count: usize,
    landing: Vec<Landing>,
    list: Vec<Trace>,
}

// Each round scans new configurations, carries traces back across the events they have not
// crossed, and fires new identities. The pure parts run in parallel and every merge happens in a
// fixed order, so the exploration is the same for any number of workers.
#[derive(Default)]
struct Round {
    fresh: Vec<usize>,
    changed: Vec<usize>,
    retry: Vec<(Identity, usize, usize)>,
}

enum Outcome {
    Applied {
        hash: u64,
        state: State,
        flow: Box<Passage>,
    },
    Blocked,
}

enum Settled {
    Blocked,
    Existing(usize, Box<Passage>),
    Repeat(usize, Box<Passage>),
    New(u64, Arc<State>, Box<Passage>),
}

// Each trace remembers the event it identifies, so the direct pass reads it instead of forming
// the identity again; a trace whose identity was blocked is formed again if needed.
#[derive(Clone, Copy)]
enum Link {
    Absent,
    Event(usize),
    Unresolved,
}

enum Resolution {
    Absent,
    Known(usize),
    Blocked,
    Pending(usize),
}

struct Candidate {
    index: usize,
    pending: Vec<(Identity, usize)>,
    resolution: Vec<Resolution>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Disagreement {
    Closed { interpreter: bool, laser: bool },
    Configuration { missing: usize, extra: usize },
    Event { missing: usize, extra: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct Summary {
    pub closed: bool,
    pub state: usize,
    pub event: usize,
    pub inferred: usize,
    pub work: usize,
}

// A configuration is scanned before it is the source of any event, so its own matches come first
// in its traces and a count marks them.
pub struct Laser {
    program: Arc<Program>,
    catalog: Arc<Catalog>,
    pub(crate) state: Vec<Arc<State>>,
    space: Space,
    event: Vec<Event>,
    identity: Vec<IndexMap<Identity, usize, Builder>>,
    incoming: Vec<Vec<usize>>,
    trace: Vec<IndexSet<Trace, Builder>>,
    origin: Vec<usize>,
    progress: Vec<Progress>,
    link: Vec<Vec<Link>>,
    passage: Vec<Passage>,
    crossed: Vec<Vec<Option<NonZeroU32>>>,
    environment: Mutex<HashMap<(usize, usize), Arc<Canonical>, Builder>>,
    pool: Pool,
    blocked: HashMap<Identity, (usize, usize), Builder>,
    round: Round,
    limit: Limit,
    work: usize,
}

fn map<Input: Send, Output: Send>(
    executor: Option<&Executor>,
    input: Vec<Input>,
    operation: impl Fn(Input) -> Output + Sync + Send,
) -> Vec<Output> {
    match executor {
        Some(executor) => executor.map(input, operation),
        None => input.into_iter().map(operation).collect(),
    }
}

fn land(position: usize) -> Option<NonZeroU32> {
    Some(
        NonZeroU32::new(u32::try_from(position + 1).expect("fewer than 2^32 traces"))
            .expect("a position after zero"),
    )
}

fn settle(landing: Vec<Landing>, position: &[usize]) -> Vec<Option<NonZeroU32>> {
    landing
        .into_iter()
        .map(|landing| match landing {
            Landing::Lost => None,
            Landing::Known(found) => land(found),
            Landing::New(index) => land(position[index]),
        })
        .collect()
}

impl Laser {
    pub fn new(source: &frontend::source::Program) -> Self {
        let program = Arc::new(Program::new(source));
        let initial = State::initial(&program);
        let mut laser = Self {
            catalog: Arc::new(Catalog::new(&program)),
            program,
            state: Vec::new(),
            space: Space::default(),
            event: Vec::new(),
            identity: Vec::new(),
            incoming: Vec::new(),
            trace: Vec::new(),
            origin: Vec::new(),
            progress: Vec::new(),
            link: Vec::new(),
            passage: Vec::new(),
            crossed: Vec::new(),
            environment: Mutex::new(HashMap::default()),
            pool: Pool::default(),
            blocked: HashMap::default(),
            round: Round::default(),
            limit: Limit::default(),
            work: 0,
        };
        let state = Arc::new(initial.canonical().state);
        let index = laser.push(state.clone());
        laser
            .space
            .admit(None, vec![(space::hash(&state), state, index)]);
        laser.round.fresh.push(index);
        laser
    }

    pub fn run(&mut self, budget: usize, limit: Limit) {
        self.execute(None, budget, limit);
    }

    pub fn parallel(&mut self, executor: &Executor, budget: usize, limit: Limit) {
        self.execute(Some(executor), budget, limit);
    }

    pub fn closed(&self) -> bool {
        self.idle() && self.blocked.is_empty()
    }

    pub fn summary(&self) -> Summary {
        Summary {
            closed: self.closed(),
            state: self.state.len(),
            event: self.event.len(),
            inferred: self.event.iter().filter(|event| !event.direct).count(),
            work: self.work,
        }
    }

    fn idle(&self) -> bool {
        self.round.fresh.is_empty() && self.round.changed.is_empty() && self.round.retry.is_empty()
    }

    fn execute(&mut self, executor: Option<&Executor>, budget: usize, limit: Limit) {
        if limit != self.limit {
            self.limit = limit;
            let blocked = std::mem::take(&mut self.blocked);
            self.round.retry.extend(
                blocked
                    .into_iter()
                    .map(|(identity, (source, position))| (identity, source, position)),
            );
            self.round
                .retry
                .sort_unstable_by_key(|(_, source, position)| (*source, *position));
        }
        let open = !self.closed();
        let mut remaining = budget;
        while remaining > 0 && !self.idle() {
            let work = self.step(executor);
            self.work += work;
            remaining = remaining.saturating_sub(work.max(1));
        }
        if open && self.closed() {
            self.passage = Vec::new();
            self.pool.release();
            direct::mark(self, executor);
            self.trace = Vec::new();
            self.link = Vec::new();
            self.crossed = Vec::new();
        }
    }

    fn push(&mut self, state: Arc<State>) -> usize {
        self.state.push(state);
        self.identity.push(IndexMap::default());
        self.incoming.push(Vec::new());
        self.trace.push(IndexSet::default());
        self.origin.push(0);
        self.progress.push(Progress::default());
        self.link.push(Vec::new());
        self.state.len() - 1
    }

    fn seed(&mut self, index: usize, found: &[scan::Match]) {
        let state = self.state[index].clone();
        for found in found {
            let capture = Capture::new(found.owner, &state, index, &mut self.pool);
            if let Some(trace) = Trace::initial(found, &state, capture, &mut self.pool) {
                self.trace[index].insert(trace);
            }
        }
        self.origin[index] = self.trace[index].len();
    }

    fn plan(&mut self, changed: Vec<usize>) -> Vec<Crossing> {
        let mut crossing = Vec::new();
        for index in changed {
            let before = self.progress[index];
            let after = Progress {
                trace: self.trace[index].len(),
                event: self.incoming[index].len(),
            };
            for (position, &event) in self.incoming[index].iter().enumerate() {
                let start = if position < before.event {
                    before.trace
                } else {
                    0
                };
                if start < after.trace {
                    crossing.push(Crossing {
                        event,
                        range: start..after.trace,
                    });
                }
            }
            self.progress[index] = after;
        }
        crossing
    }

    fn need(&self, crossing: &Crossing) -> (usize, Demand) {
        let event = &self.event[crossing.event];
        let set = &self.trace[event.target];
        let mut demand = Demand::default();
        for position in crossing.range.clone() {
            set[position].need(
                crossing.event,
                &self.passage[crossing.event],
                &self.pool,
                &mut demand,
            );
        }
        (crossing.event, demand)
    }

    fn cross(&self, crossing: Crossing) -> Carried {
        let event = &self.event[crossing.event];
        let set = &self.trace[event.target];
        let known = &self.trace[event.source];
        let mut count = 0;
        let mut list = Vec::new();
        let landing = crossing
            .range
            .map(|position| {
                let flow = &self.passage[crossing.event];
                let Some(carried) = set[position].carry(crossing.event, flow, &self.pool) else {
                    return Landing::Lost;
                };
                count += 1;
                if let Some(found) = known.get_index_of(&carried) {
                    return Landing::Known(found);
                }
                list.push(carried);
                Landing::New(list.len() - 1)
            })
            .collect();
        Carried {
            event: crossing.event,
            source: event.source,
            count,
            landing,
            list,
        }
    }

    fn candidate(&self, index: usize, range: Range<usize>) -> Candidate {
        let mut seen = IndexMap::<Identity, usize, Builder>::default();
        let mut resolution = Vec::with_capacity(range.len());
        for position in range {
            let Some(identity) = self.identify(index, &self.trace[index][position]) else {
                resolution.push(Resolution::Absent);
                continue;
            };
            if let Some(&event) = self.identity[index].get(&identity) {
                resolution.push(Resolution::Known(event));
                continue;
            }
            if self.blocked.contains_key(&identity) {
                resolution.push(Resolution::Blocked);
                continue;
            }
            let entry = seen.entry(identity);
            resolution.push(Resolution::Pending(entry.index()));
            entry.or_insert(position);
        }
        Candidate {
            index,
            pending: seen.into_iter().collect(),
            resolution,
        }
    }

    fn step(&mut self, executor: Option<&Executor>) -> usize {
        let round = std::mem::take(&mut self.round);
        let mut next = Round::default();
        let scanned = round.fresh.len();
        let mut novel = self.discover(executor, round.fresh);
        let mut changed = round.changed;
        changed.extend(novel.iter().map(|(index, _)| *index));
        let (carried, grown) = self.propagate(executor, changed);
        next.changed.extend(grown.iter().map(|(index, _)| *index));
        novel.extend(grown);
        let fired = self.fire(executor, novel, round.retry, &mut next);
        self.round = next;
        scanned + carried + fired
    }

    fn discover(
        &mut self,
        executor: Option<&Executor>,
        fresh: Vec<usize>,
    ) -> Vec<(usize, Range<usize>)> {
        let scanned = map(executor, fresh, |index| {
            (index, scan::scan(&self.catalog, &self.state[index]))
        });
        scanned
            .into_iter()
            .map(|(index, found)| {
                self.seed(index, &found);
                (index, 0..self.origin[index])
            })
            .collect()
    }

    fn propagate(
        &mut self,
        executor: Option<&Executor>,
        mut changed: Vec<usize>,
    ) -> (usize, Vec<(usize, Range<usize>)>) {
        changed.sort_unstable();
        changed.dedup();
        let crossing = self.plan(changed);
        let demand = map(executor, crossing.iter().collect(), |crossing| {
            self.need(crossing)
        });
        let passage = &self.passage;
        self.pool.prepare(executor, demand, |index| &passage[index]);
        let chunk = crossing
            .into_iter()
            .flat_map(|crossing| {
                let end = crossing.range.end;
                crossing.range.step_by(CHUNK).map(move |first| Crossing {
                    event: crossing.event,
                    range: first..(first + CHUNK).min(end),
                })
            })
            .collect();
        let carried = map(executor, chunk, |crossing| self.cross(crossing));
        let mut count = 0;
        let mut event = Vec::with_capacity(carried.len());
        let mut landing = Vec::with_capacity(carried.len());
        let mut group = BTreeMap::<usize, Vec<(usize, Carried)>>::new();
        for (position, value) in carried.into_iter().enumerate() {
            count += value.count;
            event.push(value.event);
            if value.list.is_empty() {
                landing.push(settle(value.landing, &[]));
            } else {
                landing.push(Vec::new());
                group
                    .entry(value.source)
                    .or_default()
                    .push((position, value));
            }
        }
        let taken = group
            .into_iter()
            .map(|(index, list)| (index, std::mem::take(&mut self.trace[index]), list))
            .collect::<Vec<_>>();
        let inserted = map(executor, taken, |(index, mut set, list)| {
            let start = set.len();
            let settled = list
                .into_iter()
                .map(|(position, carried)| {
                    let found = carried
                        .list
                        .into_iter()
                        .map(|trace| set.insert_full(trace).0)
                        .collect::<Vec<_>>();
                    (position, settle(carried.landing, &found))
                })
                .collect::<Vec<_>>();
            (index, start, set, settled)
        });
        let mut grown = Vec::new();
        for (index, start, set, settled) in inserted {
            let end = set.len();
            self.trace[index] = set;
            if start < end {
                grown.push((index, start..end));
            }
            for (position, value) in settled {
                landing[position] = value;
            }
        }
        for (event, landing) in event.into_iter().zip(landing) {
            self.crossed[event].extend(landing);
        }
        (count, grown)
    }

    fn fire(
        &mut self,
        executor: Option<&Executor>,
        novel: Vec<(usize, Range<usize>)>,
        retry: Vec<(Identity, usize, usize)>,
        next: &mut Round,
    ) -> usize {
        let candidate = map(executor, novel, |(index, range)| {
            self.candidate(index, range)
        });
        let revisit = retry
            .iter()
            .map(|&(_, source, position)| (source, position))
            .collect::<Vec<_>>();
        let mut pending = retry;
        let mut resolved = Vec::with_capacity(candidate.len());
        for value in candidate {
            resolved.push((value.index, pending.len(), value.resolution));
            pending.extend(
                value
                    .pending
                    .into_iter()
                    .map(|(identity, position)| (identity, value.index, position)),
            );
        }
        let count = pending.len();
        let outcome = map(
            executor,
            pending.iter().collect(),
            |(identity, source, position)| {
                self.apply(identity, *source, &self.trace[*source][*position])
            },
        );
        let created = self.create(executor, pending, outcome, next);
        for (index, offset, resolution) in resolved {
            self.link[index].extend(resolution.into_iter().map(|value| match value {
                Resolution::Absent => Link::Absent,
                Resolution::Known(event) => Link::Event(event),
                Resolution::Blocked => Link::Unresolved,
                Resolution::Pending(slot) => {
                    created[offset + slot].map_or(Link::Unresolved, Link::Event)
                }
            }));
        }
        for ((source, position), &event) in revisit.into_iter().zip(&created) {
            if let Some(event) = event {
                self.link[source][position] = Link::Event(event);
            }
        }
        count
    }

    fn create(
        &mut self,
        executor: Option<&Executor>,
        pending: Vec<(Identity, usize, usize)>,
        outcome: Vec<Outcome>,
        next: &mut Round,
    ) -> Vec<Option<usize>> {
        let item = outcome
            .iter()
            .filter_map(|outcome| match outcome {
                Outcome::Applied { hash, state, .. } => Some((*hash, state)),
                Outcome::Blocked => None,
            })
            .collect::<Vec<_>>();
        let mut found = self.space.resolve(executor, &item).into_iter();
        let paired = outcome
            .into_iter()
            .map(|outcome| {
                let found = matches!(outcome, Outcome::Applied { .. })
                    .then(|| found.next().expect("one answer for each applied event"));
                (outcome, found)
            })
            .collect::<Vec<_>>();
        let settled = map(executor, paired, |(outcome, found)| {
            match (outcome, found) {
                (Outcome::Applied { flow, .. }, Some(Found::Existing(index))) => {
                    Settled::Existing(index, flow)
                }
                (Outcome::Applied { flow, .. }, Some(Found::Repeat(earlier))) => {
                    Settled::Repeat(earlier, flow)
                }
                (Outcome::Applied { hash, state, flow }, Some(Found::New)) => {
                    Settled::New(hash, Arc::new(state), flow)
                }
                _ => Settled::Blocked,
            }
        });
        let mut target = Vec::<Option<usize>>::new();
        let mut admitted = Vec::new();
        let mut created = Vec::with_capacity(pending.len());
        let mut fired = Vec::new();
        for ((identity, source, position), settled) in pending.into_iter().zip(settled) {
            let (resolved, flow) = match settled {
                Settled::Blocked => {
                    self.blocked.insert(identity, (source, position));
                    created.push(None);
                    continue;
                }
                Settled::Existing(index, flow) => (Some(index), flow),
                Settled::Repeat(earlier, flow) => (target[earlier], flow),
                Settled::New(..) if self.state.len() >= self.limit.configuration => {
                    target.push(None);
                    self.blocked.insert(identity, (source, position));
                    created.push(None);
                    continue;
                }
                Settled::New(hash, state, flow) => {
                    let index = self.push(state.clone());
                    admitted.push((hash, state, index));
                    next.fresh.push(index);
                    (Some(index), flow)
                }
            };
            target.push(resolved);
            let Some(resolved) = resolved else {
                self.blocked.insert(identity, (source, position));
                created.push(None);
                continue;
            };
            let event = self.event.len();
            self.crossed.push(Vec::new());
            self.passage.push(*flow);
            fired.push((source, event, identity));
            self.event.push(Event {
                source,
                slot: 0,
                target: resolved,
                direct: position < self.origin[source],
            });
            self.incoming[resolved].push(event);
            next.changed.push(resolved);
            created.push(Some(event));
        }
        self.space.admit(executor, admitted);
        fired.sort_by_key(|&(source, event, _)| (source, event));
        let mut group = Vec::<(usize, Vec<(Identity, usize)>)>::new();
        for (source, event, identity) in fired {
            match group.last_mut() {
                Some((index, list)) if *index == source => list.push((identity, event)),
                _ => group.push((source, vec![(identity, event)])),
            }
        }
        for (index, list) in &group {
            let base = self.identity[*index].len();
            for (offset, &(_, event)) in list.iter().enumerate() {
                self.event[event].slot = base + offset;
            }
        }
        let taken = group
            .into_iter()
            .map(|(index, list)| (index, std::mem::take(&mut self.identity[index]), list))
            .collect::<Vec<_>>();
        let inserted = map(executor, taken, |(index, mut table, list)| {
            table.extend(list);
            (index, table)
        });
        for (index, table) in inserted {
            self.identity[index] = table;
        }
        created
    }

    fn canonical(&self, origin: usize, frame: usize) -> Arc<Canonical> {
        if let Some(found) = self
            .environment
            .lock()
            .expect("an unpoisoned cache")
            .get(&(origin, frame))
        {
            return found.clone();
        }
        let canonical = Arc::new(self.state[origin].environment(frame));
        self.environment
            .lock()
            .expect("an unpoisoned cache")
            .entry((origin, frame))
            .or_insert(canonical)
            .clone()
    }

    fn identify(&self, source: usize, trace: &Trace) -> Option<Identity> {
        let binding = trace.binding(&self.state[source], &self.pool)?;
        let owner = match (trace.current(), &trace.capture) {
            (Some(frame), _) => Owner::Frame(frame),
            (None, Some(capture)) => {
                let canonical = self.canonical(capture.origin, capture.frame);
                Owner::Capture(Arc::new(capture.environment(&canonical)))
            }
            (None, None) => unreachable!("the root frame exists in every configuration"),
        };
        Some(Identity {
            source,
            frame: trace.frame,
            owner,
            rule: trace.rule,
            binding,
        })
    }

    fn identity(&self, event: usize) -> &Identity {
        let value = &self.event[event];
        self.identity[value.source]
            .get_index(value.slot)
            .expect("every event keeps its identity")
            .0
    }

    fn find(&self, source: usize, trace: &Trace) -> Option<usize> {
        let identity = self.identify(source, trace)?;
        self.identity[source].get(&identity).copied()
    }

    fn apply(&self, identity: &Identity, source: usize, trace: &Trace) -> Outcome {
        let flow = match &trace.capture {
            Some(capture) if capture.current.is_none() => Some((
                capture.origin,
                capture.frame,
                capture.flow(&self.state[capture.origin], &self.pool),
            )),
            _ => None,
        };
        let owner = match (&identity.owner, &flow) {
            (Owner::Frame(frame), _) => Owner::Frame(*frame),
            (Owner::Capture(_), Some((origin, frame, flow))) => Owner::Capture(Closure {
                state: &self.state[*origin],
                flow,
                capture: *frame,
            }),
            (Owner::Capture(_), None) => unreachable!("a captured owner carries its attachment"),
        };
        let result = crate::application::apply(crate::application::Request {
            source: &self.state[source],
            scope: &self.program.scope,
            frame: identity.frame,
            owner,
            rule: &self.program.rule[identity.rule],
            binding: &identity.binding,
        });
        if !self.limit.admits(
            result.state.world.len(),
            result.state.size(),
            result.state.reachable().len(),
        ) {
            return Outcome::Blocked;
        }
        let canonical = result.state.canonical();
        let applied = result.flow.rename(canonical);
        Outcome::Applied {
            hash: space::hash(&applied.state),
            state: applied.state,
            flow: Box::new(Passage::new(applied.flow)),
        }
    }
}

#[cfg(test)]
#[path = "test/laser.rs"]
mod test;
