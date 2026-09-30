mod agreement;
mod capture;
mod carriage;
mod component;
mod creation;
mod deduction;
mod direct;
mod discovery;
mod enclosure;
pub mod ending;
mod firing;
mod focus;
mod forest;
mod independence;
mod layout;
pub mod makeup;
mod memo;
pub mod net;
mod passage;
mod pool;
pub mod report;
mod scan;
mod shard;
pub mod size;
mod space;
mod support;
mod taxonomy;
mod trace;
mod transition;

use crate::application::Owner;
use crate::catalog::Catalog;
use crate::executor::Executor;
use crate::flow::Binding;
use crate::prism::{Outcome, Verdict};
use crate::program::Program;
use crate::runtime::Limit;
use crate::state::{Canonical, State};
use crate::status::Status;
use capture::Environment;
use deduction::Deduction;
use hashing::Builder;
use independence::Independence;
use indexmap::{IndexMap, IndexSet};
use layout::Layout;
use makeup::Makeup;
use memo::Memo;
use passage::Passage;
use pool::Pool;
use space::Space;
use std::collections::{HashMap, HashSet};
use std::num::NonZeroU32;
use std::sync::Arc;
use taxonomy::Taxonomy;
use trace::Trace;
use transition::{Effect, Key};

const CHUNK: usize = 256;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Identity {
    source: usize,
    frame: usize,
    owner: Owner<Arc<Environment>>,
    rule: usize,
    binding: Binding,
}

// An event is matched when a match found at its own source identifies it; marking closed walks can
// make more events direct, never more matched.
struct Event {
    source: usize,
    slot: usize,
    target: usize,
    direct: bool,
    matched: bool,
}

// The traces of a configuration before the first count have crossed its incoming events before
// the second, so each trace crosses each event once.
#[derive(Clone, Copy, Default)]
struct Progress {
    trace: usize,
    event: usize,
}

// Each round scans new configurations, carries traces back across the events they have not
// crossed, and fires new identities. The pure parts run in parallel and every merge happens in a
// fixed order, so the exploration is the same for any number of workers. A budget can end a round
// early; the configurations it did not scan, the crossings it did not carry and the identities it
// did not fire come first in the next.
#[derive(Default)]
struct Round {
    fresh: Vec<usize>,
    changed: Vec<usize>,
    crossing: Vec<carriage::Crossing>,
    retry: Vec<(Identity, usize)>,
}

// Each trace remembers the event it identifies, so the passes after closing read it instead of
// forming the identity again; a trace whose identity was blocked is formed again if needed.
#[derive(Clone, Copy)]
enum Link {
    Absent,
    Event(usize),
    Unresolved,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Disagreement {
    Closed { reference: bool, laser: bool },
    Endless { reference: bool, laser: bool },
    Configuration { missing: usize, extra: usize },
    Event { missing: usize, extra: usize },
    Support { configuration: usize },
    Work { reference: usize, laser: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Summary {
    pub closed: bool,
    pub state: usize,
    pub event: usize,
    pub inferred: usize,
    pub work: usize,
}

// A configuration is scanned before it is the source of any event, so its own matches come first
// in its traces and a count marks them. Every later trace keeps the event and the position it was
// first carried from.
pub struct Laser {
    program: Arc<Program>,
    catalog: Arc<Catalog>,
    state: Vec<Arc<State>>,
    makeup: Vec<Arc<Makeup>>,
    layout: Vec<Option<Arc<Layout>>>,
    taxonomy: Taxonomy,
    memo: HashMap<Key, Arc<Effect>, Builder>,
    whole: HashSet<Key, Builder>,
    space: Space,
    event: Vec<Event>,
    identity: Vec<IndexMap<Identity, usize, Builder>>,
    incoming: Vec<Vec<usize>>,
    trace: Vec<IndexSet<Trace, Builder>>,
    origin: Vec<usize>,
    parent: Vec<Vec<(u32, u32)>>,
    progress: Vec<Progress>,
    link: Vec<Vec<Link>>,
    passage: Vec<Passage>,
    crossed: Vec<Vec<Option<NonZeroU32>>>,
    capture: capture::Store,
    environment: Memo<(usize, usize), Arc<Canonical>>,
    pool: Pool,
    blocked: HashMap<Identity, usize, Builder>,
    support: Option<support::Support>,
    deduction: Deduction,
    round: Round,
    limit: Limit,
    work: usize,
    traced: usize,
    plain: bool,
    independence: Option<Independence>,
    peak: usize,
}

// A configuration made from its makeup, with the layout of its parts when it is split into them.
fn build(taxonomy: &Taxonomy, makeup: &Makeup) -> (Arc<State>, Option<Arc<Layout>>) {
    let state = Arc::new(taxonomy.materialize(makeup));
    let layout = taxonomy
        .split(makeup)
        .then(|| Arc::new(Layout::new(taxonomy, makeup)));
    (state, layout)
}

// Changes some of a list of tables in parallel: each named table is taken out, changed with its
// input by one worker and put back.
fn update<Table: Default + Send, Input: Send>(
    executor: Option<&Executor>,
    table: &mut [Table],
    input: Vec<(usize, Input)>,
    change: impl Fn(&mut Table, Input) + Sync + Send,
) {
    let taken = input
        .into_iter()
        .map(|(index, input)| (index, std::mem::take(&mut table[index]), input))
        .collect();
    let changed = map(executor, taken, |(index, mut value, input)| {
        change(&mut value, input);
        (index, value)
    });
    for (index, value) in changed {
        table[index] = value;
    }
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

impl Laser {
    pub fn new(source: &frontend::source::Program) -> Self {
        Self::begin(source, false)
    }

    // A plain exploration fires only what each configuration's own matches identify, so it
    // reaches every schedule of plain events and no inferred event: nothing is carried back.
    pub fn plain(source: &frontend::source::Program) -> Self {
        Self::begin(source, true)
    }

    // A reduced exploration fires, at each configuration, the events of one part that commute with
    // every other event, and reaches every configuration where a plain run ends.
    pub fn reduced(source: &frontend::source::Program) -> Self {
        let mut laser = Self::begin(source, true);
        laser.independence = Independence::new(&laser.program);
        laser
    }

    fn begin(source: &frontend::source::Program, plain: bool) -> Self {
        let program = Arc::new(Program::new(source));
        let initial = State::initial(&program);
        let mut laser = Self {
            catalog: Arc::new(Catalog::new(&program)),
            program,
            state: Vec::new(),
            makeup: Vec::new(),
            layout: Vec::new(),
            taxonomy: Taxonomy::default(),
            memo: HashMap::default(),
            whole: HashSet::default(),
            space: Space::default(),
            event: Vec::new(),
            identity: Vec::new(),
            incoming: Vec::new(),
            trace: Vec::new(),
            origin: Vec::new(),
            parent: Vec::new(),
            progress: Vec::new(),
            link: Vec::new(),
            passage: Vec::new(),
            crossed: Vec::new(),
            capture: capture::Store::default(),
            environment: Memo::default(),
            pool: Pool::default(),
            blocked: HashMap::default(),
            support: None,
            deduction: Deduction::default(),
            round: Round::default(),
            limit: Limit::default(),
            work: 0,
            traced: 0,
            plain,
            independence: None,
            peak: 0,
        };
        let draft = laser.taxonomy.analyze(&initial);
        let (root, mut kind) = laser.taxonomy.intern(&draft);
        kind.sort_unstable();
        let makeup = Makeup { root, kind };
        let (state, layout) = build(&laser.taxonomy, &makeup);
        let makeup = Arc::new(makeup);
        let index = laser.push(state, makeup.clone(), layout);
        laser
            .space
            .admit(None, vec![(space::hash(&makeup), makeup, index)]);
        laser.round.fresh.push(index);
        laser.peak = laser.retained();
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

    pub fn status(&self) -> (Vec<Status>, Vec<Status>) {
        let found;
        let support = match &self.support {
            Some(support) => support,
            None => {
                found = support::establish(self, None);
                &found
            }
        };
        let status = |value: &bool| {
            if *value {
                Status::Supported
            } else {
                Status::Unsupported
            }
        };
        (
            support.state.iter().map(status).collect(),
            support.event.iter().map(status).collect(),
        )
    }

    pub fn verdict(&self, target: &frontend::source::Program) -> Verdict {
        let state = State::target(&self.program, target);
        let candidate = self
            .taxonomy
            .find(&state)
            .and_then(|makeup| self.space.find(&makeup));
        let (status, _) = self.status();
        let outcome = match candidate.map(|index| status[index]) {
            Some(Status::Supported) => Outcome::Reached,
            _ if self.closed() => Outcome::Unreachable,
            _ => Outcome::Unknown,
        };
        Verdict {
            outcome,
            witness: candidate.filter(|_| outcome == Outcome::Reached),
        }
    }

    fn idle(&self) -> bool {
        self.round.fresh.is_empty()
            && self.round.changed.is_empty()
            && self.round.crossing.is_empty()
            && self.round.retry.is_empty()
    }

    fn execute(&mut self, executor: Option<&Executor>, budget: usize, limit: Limit) {
        // A blocked identity stays blocked until its retry fires it, so a trace that identifies it
        // meanwhile waits for that event instead of firing it a second time.
        if limit != self.limit {
            self.limit = limit;
            self.round.retry = self
                .blocked
                .iter()
                .map(|(identity, &position)| (identity.clone(), position))
                .collect();
            self.round
                .retry
                .sort_unstable_by_key(|(identity, position)| (identity.source, *position));
        }
        let open = !self.closed();
        let mut remaining = budget;
        while remaining > 0 && !self.idle() && self.retained() < self.limit.record {
            let work = self.step(executor, remaining);
            self.work += work;
            self.peak = self.peak.max(self.retained());
            remaining = remaining.saturating_sub(work.max(1));
        }
        if open && self.closed() {
            self.close(executor);
        }
    }

    // Closing settles which events are direct, what is supported and the deductions, then keeps
    // what later questions read and releases what only traces reach: the traces, their links,
    // crossings, captures and bases, the environments they identify with and the transitions
    // firing learned. The images go first, before the closing passes allocate.
    fn close(&mut self, executor: Option<&Executor>) {
        self.pool.release();
        direct::mark(self, executor);
        self.support = Some(support::establish(self, executor));
        self.deduction = Deduction::derive(self);
        self.trace = Vec::new();
        self.traced = 0;
        self.parent = Vec::new();
        self.link = Vec::new();
        self.crossed = Vec::new();
        self.capture = capture::Store::default();
        self.pool = Pool::default();
        self.environment = Memo::default();
        self.memo = HashMap::default();
        self.whole = HashSet::default();
    }

    // A record is a configuration, an event or a trace, the parts that grow with exploration;
    // closing releases the traces.
    fn retained(&self) -> usize {
        self.state.len() + self.event.len() + self.traced
    }

    fn push(
        &mut self,
        state: Arc<State>,
        makeup: Arc<Makeup>,
        layout: Option<Arc<Layout>>,
    ) -> usize {
        self.state.push(state);
        self.makeup.push(makeup);
        self.layout.push(layout);
        self.identity.push(IndexMap::default());
        self.incoming.push(Vec::new());
        self.trace.push(IndexSet::default());
        self.origin.push(0);
        self.parent.push(Vec::new());
        self.progress.push(Progress::default());
        self.link.push(Vec::new());
        self.state.len() - 1
    }

    // A round that takes at most the allowance of work: each phase takes what the phases before it
    // left, and stops once it has taken its share or the retained records reach the record limit.
    fn step(&mut self, executor: Option<&Executor>, allowance: usize) -> usize {
        let round = std::mem::take(&mut self.round);
        let (mut novel, fresh) = self.discover(executor, round.fresh, allowance);
        let mut next = Round {
            fresh,
            ..Round::default()
        };
        let scanned = novel.len();
        let mut changed = round.changed;
        changed.extend(novel.iter().map(|(index, _)| *index));
        let carried = if self.plain {
            0
        } else {
            let carriage = self.propagate(executor, round.crossing, changed, allowance - scanned);
            next.changed
                .extend(carriage.grown.iter().map(|(index, _)| *index));
            next.crossing = carriage.rest;
            novel.extend(carriage.grown);
            carriage.count
        };
        let rest = allowance - scanned - carried;
        let fired = self.fire(executor, novel, round.retry, &mut next, rest);
        if self.plain {
            next.changed.clear();
        }
        self.round = next;
        scanned + carried + fired
    }
}

#[cfg(test)]
#[path = "test/laser.rs"]
mod test;
