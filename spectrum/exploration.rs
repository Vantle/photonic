use crate::budget::Budget;
use crate::configuration::{Coherence, Configuration, Frame, Occurrence, Opener, Value};
use crate::extent::Extent;
use crate::numbering::Numbering;
use crate::order::{self, Canonical, Naming};
use crate::recording::{Engine, Mode, Order};
use frontend::source::{Definition, Program};
use photonic::executor::Executor;
use photonic::laser::Laser;
use photonic::laser::ending;
use photonic::place::Place;
use photonic::prism::{Outcome, Reach, Verdict};
use photonic::runtime::Runtime;
use photonic::snapshot::{self, Link, Node};
use photonic::status::Status;
use photonic::stop::Stop;
use std::collections::VecDeque;
use std::sync::OnceLock;

// Explorations run on every core the machine has, through one pool the process keeps; both engines
// explore identically on any number of threads, so no answer depends on it.
fn executor() -> Option<&'static Executor> {
    static POOL: OnceLock<Option<Executor>> = OnceLock::new();
    POOL.get_or_init(|| Executor::new(std::thread::available_parallelism().ok()?).ok())
        .as_ref()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rule {
    pub text: String,
    pub definition: Definition,
    pub canonical: Definition,
    pub scope: Option<Opener>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Event {
    pub source: usize,
    pub target: usize,
    pub rule: usize,
    pub supported: bool,
    pub footprint: Vec<Place>,
    pub exact: Vec<Place>,
    pub read: Vec<Place>,
    pub world: Vec<usize>,
    pub deduction: Vec<usize>,
}

// What an exploration keeps to answer verdicts and place maps later: the interpreter's are
// recorded as it runs, Laser answers them itself, and a direct path keeps neither.
enum Explorer {
    Path,
    Interpreter {
        reach: Reach,
        resource: Vec<Vec<Link>>,
    },
    Laser(Box<Laser>),
}

pub struct Exploration {
    pub key: String,
    pub mode: Mode,
    pub engine: Engine,
    pub order: Order,
    pub shape: Option<u64>,
    pub naming: Naming,
    pub program: Program,
    pub closed: bool,
    pub stop: Vec<Stop>,
    pub reached: bool,
    pub work: usize,
    pub rule: Vec<Rule>,
    pub configuration: Vec<Configuration>,
    pub event: Vec<Event>,
    pub outgoing: Vec<Vec<usize>>,
    pub incoming: Vec<Vec<usize>>,
    pub parent: Vec<Option<usize>>,
    pub depth: Vec<Option<usize>>,
    explorer: Explorer,
    numbering: Option<Numbering>,
}

// Everything an exploration depends on: the whole program with its scopes, the naming its handles
// use, the mode, engine and budget, and the goal of a path. Its key hashes it, and the store
// compares it whole on every hit, since two identities can share a key.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Identity {
    pub program: Program,
    pub naming: Naming,
    pub mode: Mode,
    pub engine: Engine,
    pub budget: Budget,
    pub goal: Option<Program>,
}

pub struct Plan {
    pub order: Order,
    pub shape: Option<u64>,
    pub identity: Identity,
    pub key: String,
}

struct Record<'report> {
    closed: bool,
    stop: Vec<Stop>,
    reached: bool,
    work: usize,
    definition: &'report [snapshot::Definition],
    state: &'report [Node],
    event: Vec<Event>,
}

pub fn id(place: Place) -> usize {
    let (Place::World(_, id) | Place::Context(_, id) | Place::Held(_, id)) = place;
    id
}

fn occurrence(token: &snapshot::Token) -> Occurrence {
    Occurrence {
        id: token.id,
        value: match &token.value {
            snapshot::Value::Atom(atom) => Value::Atom(atom.to_string()),
            snapshot::Value::Rule(rule) => Value::Rule(*rule),
        },
    }
}

// Configurations are numbered in the canonical program's own names, so renaming atoms keeps every
// handle, and only then shown in the program's names.
pub(crate) fn show(configuration: Configuration, naming: &Naming) -> Configuration {
    let occurrence = |value: Occurrence| Occurrence {
        value: match value.value {
            Value::Atom(atom) => Value::Atom(naming.name(&atom).to_owned()),
            rule => rule,
        },
        ..value
    };
    Configuration {
        coherence: configuration
            .coherence
            .into_iter()
            .map(|coherence| Coherence {
                occurrence: coherence.occurrence.into_iter().map(occurrence).collect(),
                ..coherence
            })
            .collect(),
        frame: configuration
            .frame
            .into_iter()
            .map(|frame| Frame {
                rule: frame.rule.into_iter().map(occurrence).collect(),
                held: frame.held.into_iter().map(occurrence).collect(),
                ..frame
            })
            .collect(),
        ..configuration
    }
}

pub(crate) fn configuration(node: &Node) -> Configuration {
    Configuration {
        coherence: node
            .world
            .iter()
            .map(|world| Coherence {
                frame: world.frame,
                occurrence: world.particle.iter().map(occurrence).collect(),
            })
            .collect(),
        frame: node
            .frame
            .iter()
            .map(|frame| Frame {
                opener: match (frame.opener, frame.parent) {
                    (Some(rule), _) => Some(Opener::Rule(rule)),
                    (None, Some(_)) => Some(Opener::Program),
                    (None, None) => None,
                },
                parent: frame.parent,
                lexical: frame.lexical,
                rule: frame.particle.iter().map(occurrence).collect(),
                held: frame.held.iter().map(occurrence).collect(),
            })
            .collect(),
        supported: node.status == Status::Supported,
    }
}

pub(crate) fn rule(definition: &snapshot::Definition, naming: &Naming) -> Rule {
    let plain = naming.show(&definition.rule);
    Rule {
        text: frontend::text::definition(&plain),
        canonical: plain.canonical(),
        definition: plain,
        scope: None,
    }
}

// An exact target in the canonical program's names, which also lists every root rule of the
// program when preserve asks for them.
pub(crate) fn hide(
    naming: &Naming,
    program: &Program,
    target: &Program,
    preserve: bool,
) -> Program {
    let mut hidden = naming.hide(target);
    if preserve {
        hidden.preserve(program);
    }
    hidden
}

// The breadth-first tree of supported events from the start through the configurations admit
// accepts: each configuration's parent event and depth, none where the tree does not reach.
pub(crate) fn tree(
    outgoing: &[Vec<usize>],
    event: &[Event],
    admit: impl Fn(usize) -> bool,
) -> (Vec<Option<usize>>, Vec<Option<usize>>) {
    let count = outgoing.len();
    let mut parent = vec![None; count];
    let mut depth = vec![None; count];
    if count == 0 || !admit(0) {
        return (parent, depth);
    }
    depth[0] = Some(0);
    let mut queue = VecDeque::from([(0, 0)]);
    while let Some((current, level)) = queue.pop_front() {
        for &index in &outgoing[current] {
            let target = event[index].target;
            if !event[index].supported || !admit(target) || depth[target].is_some() {
                continue;
            }
            depth[target] = Some(level + 1);
            parent[target] = Some(index);
            queue.push_back((target, level + 1));
        }
    }
    (parent, depth)
}

// The events from the start to a configuration a tree reaches, read back along parent events.
pub(crate) fn trail(parent: &[Option<usize>], event: &[Event], node: usize) -> Vec<usize> {
    let mut trail = std::iter::successors(parent[node], |&index| parent[event[index].source])
        .collect::<Vec<_>>();
    trail.reverse();
    trail
}

impl Plan {
    pub fn new(
        program: &Program,
        mode: Mode,
        engine: Engine,
        budget: Budget,
        goal: Option<Program>,
    ) -> Self {
        let Canonical {
            order,
            shape,
            program,
            naming,
        } = match mode {
            Mode::Exhaustive | Mode::Plain => order::exhaustive(program),
            Mode::Path => order::source(program),
        };
        let goal = goal.map(|goal| {
            let mut hidden = naming.hide(&goal);
            hidden.rule.sort();
            hidden
        });
        let identity = Identity {
            program,
            naming,
            mode,
            engine,
            budget,
            goal,
        };
        Self {
            order,
            shape,
            key: format!("{:016x}", hashing::value(&identity)),
            identity,
        }
    }
}

impl Exploration {
    pub fn new(plan: Plan) -> Self {
        match (plan.identity.mode, plan.identity.engine) {
            (Mode::Exhaustive, Engine::Interpreter) => Self::interpret(plan),
            (Mode::Path, _) => Self::walk(plan),
            _ => Self::compile(plan),
        }
    }

    fn interpret(plan: Plan) -> Self {
        let budget = plan.identity.budget;
        let mut runtime = Runtime::new(&plan.identity.program);
        match executor() {
            Some(executor) => runtime.parallel(executor, budget.work, budget.limit()),
            None => runtime.run(budget.work, budget.limit()),
        }
        let snapshot = runtime.snapshot();
        let event = snapshot
            .event
            .iter()
            .map(|event| Event {
                source: event.source,
                target: event.target,
                rule: event.rule,
                supported: event.status == Status::Supported,
                footprint: event.footprint.clone(),
                exact: event.exact.clone(),
                read: event.read.clone(),
                world: event.world.clone(),
                deduction: snapshot.deduction(event.id),
            })
            .collect();
        let resource = snapshot
            .event
            .iter()
            .map(|event| runtime.resource(event.id).unwrap_or_default())
            .collect();
        let record = Record {
            closed: snapshot.closed,
            stop: snapshot.stop.clone(),
            reached: false,
            work: snapshot.work,
            definition: &snapshot.definition,
            state: &snapshot.state,
            event,
        };
        let explorer = Explorer::Interpreter {
            reach: runtime.reach(),
            resource,
        };
        Self::assemble(plan, record, explorer)
    }

    fn compile(plan: Plan) -> Self {
        let budget = plan.identity.budget;
        let mut laser = match plan.identity.mode {
            Mode::Plain => Laser::plain(&plan.identity.program),
            Mode::Exhaustive | Mode::Path => Laser::new(&plan.identity.program),
        };
        match executor() {
            Some(executor) => laser.parallel(executor, budget.work, budget.limit()),
            None => laser.run(budget.work, budget.limit()),
        }
        let report = laser.report();
        let record = Record {
            closed: report.closed,
            stop: report.stop.clone(),
            reached: false,
            work: report.work,
            definition: &report.definition,
            state: &report.state,
            event: report
                .event
                .into_iter()
                .map(|event| Event {
                    source: event.source,
                    target: event.target,
                    rule: event.rule,
                    supported: event.status == Status::Supported,
                    footprint: event.footprint,
                    exact: event.exact,
                    read: event.read,
                    world: event.world,
                    deduction: event.deduction,
                })
                .collect(),
        };
        Self::assemble(plan, record, Explorer::Laser(Box::new(laser)))
    }

    fn walk(plan: Plan) -> Self {
        let budget = plan.identity.budget;
        let mut search =
            photonic::path::Search::new(plan.identity.program.clone(), plan.identity.goal.clone());
        search.run(budget.work, budget.limit());
        let report = search.report();
        let event = report
            .event
            .iter()
            .map(|event| Event {
                source: event.source,
                target: event.target,
                rule: event.rule,
                supported: true,
                footprint: event.footprint.clone(),
                exact: event.exact.clone(),
                read: event.read.clone(),
                world: Vec::new(),
                deduction: Vec::new(),
            })
            .collect();
        // A direct path follows one run, so it never settles what the others do, and reached says
        // whether that run met its goal.
        let record = Record {
            closed: false,
            stop: report.stop.clone(),
            reached: report.outcome == Outcome::Reached,
            work: report.work,
            definition: &report.definition,
            state: &report.state,
            event,
        };
        Self::assemble(plan, record, Explorer::Path)
    }

    fn assemble(plan: Plan, record: Record<'_>, explorer: Explorer) -> Self {
        let naming = &plan.identity.naming;
        let configuration = record
            .state
            .iter()
            .map(self::configuration)
            .collect::<Vec<_>>();
        let numbering = (plan.identity.mode != Mode::Path)
            .then(|| Numbering::new(&configuration, &record.event));
        let (configuration, event) = match &numbering {
            Some(numbering) => numbering.apply(configuration, record.event),
            None => (configuration, record.event),
        };
        let configuration = configuration
            .into_iter()
            .map(|configuration| show(configuration, naming))
            .collect::<Vec<_>>();
        let count = configuration.len();
        let mut outgoing = vec![Vec::new(); count];
        let mut incoming = vec![Vec::new(); count];
        for (index, entry) in event.iter().enumerate() {
            outgoing[entry.source].push(index);
            incoming[entry.target].push(index);
        }
        let (parent, depth) = tree(&outgoing, &event, |_| true);
        let mut rule = record
            .definition
            .iter()
            .map(|entry| self::rule(entry, naming))
            .collect::<Vec<_>>();
        for frame in configuration
            .iter()
            .flat_map(|configuration| &configuration.frame)
        {
            let Some(opener) = frame.opener else {
                continue;
            };
            for occurrence in &frame.rule {
                if let Value::Rule(index) = occurrence.value
                    && let Some(entry) = rule.get_mut(index)
                {
                    entry.scope = Some(opener);
                }
            }
        }
        Self {
            key: plan.key,
            mode: plan.identity.mode,
            engine: plan.identity.engine,
            order: plan.order,
            shape: plan.shape,
            naming: plan.identity.naming,
            program: plan.identity.program,
            closed: record.closed,
            stop: record.stop,
            reached: record.reached,
            work: record.work,
            rule,
            configuration,
            event,
            outgoing,
            incoming,
            parent,
            depth,
            explorer,
            numbering,
        }
    }

    pub fn path(&self, configuration: usize) -> Option<Vec<usize>> {
        self.depth.get(configuration).copied().flatten()?;
        Some(trail(&self.parent, &self.event, configuration))
    }

    pub fn verdict(&self, target: &Program, preserve: bool) -> Option<Verdict> {
        let hidden = hide(&self.naming, &self.program, target, preserve);
        let verdict = match &self.explorer {
            Explorer::Path => None,
            Explorer::Interpreter { reach, .. } => Some(reach.verdict(&hidden)),
            Explorer::Laser(laser) => Some(laser.verdict(&hidden)),
        }?;
        Some(Verdict {
            witness: verdict.witness.map(|witness| {
                self.numbering
                    .as_ref()
                    .map_or(witness, |numbering| numbering.configuration(witness))
            }),
            ..verdict
        })
    }

    // Each place after an event and the places it came from before; a direct path keeps none.
    pub fn resource(&self, event: usize) -> Vec<Link> {
        let event = self
            .numbering
            .as_ref()
            .map_or(event, |numbering| numbering.event(event));
        match &self.explorer {
            Explorer::Path => Vec::new(),
            Explorer::Interpreter { resource, .. } => resource[event].clone(),
            Explorer::Laser(laser) => laser.resource(event),
        }
    }

    pub fn stuck(&self, index: usize) -> bool {
        self.configuration[index].supported
            && self.outgoing[index]
                .iter()
                .all(|&event| !self.event[event].supported)
    }

    pub fn leaf(&self) -> impl Iterator<Item = usize> + '_ {
        (0..self.configuration.len()).filter(|&index| self.stuck(index))
    }

    pub fn firing(&self, rule: usize) -> impl Iterator<Item = usize> + '_ {
        (0..self.event.len())
            .filter(move |&event| self.event[event].rule == rule && self.event[event].supported)
    }

    // A cycle of supported events through configurations that avoided marks, found from the
    // start: the configuration where it closes, and the events from the start around it.
    pub fn cycle(&self, avoided: &[bool]) -> Option<(usize, Vec<usize>)> {
        ending::cycle(self.configuration.len(), |node| {
            self.outgoing[node]
                .iter()
                .map(|&event| (event, &self.event[event]))
                .filter(|(_, entry)| entry.supported && avoided[entry.target])
                .map(|(event, entry)| (event, entry.target))
        })
    }

    // Whether a run can go on forever: yes once a cycle of supported events is found, and no once
    // the exploration closes without one.
    pub fn endless(&self) -> Option<bool> {
        if self.cycle(&vec![true; self.configuration.len()]).is_some() {
            return Some(true);
        }
        self.closed.then_some(false)
    }

    pub fn find(&self, configuration: usize, id: usize) -> Option<&Occurrence> {
        let entry = self.configuration.get(configuration)?;
        entry
            .coherence
            .iter()
            .flat_map(|coherence| &coherence.occurrence)
            .chain(
                entry
                    .frame
                    .iter()
                    .flat_map(|frame| frame.rule.iter().chain(&frame.held)),
            )
            .find(|occurrence| occurrence.id == id)
    }

    pub fn size(&self) -> usize {
        let occurrence = self
            .configuration
            .iter()
            .map(|configuration| {
                configuration
                    .coherence
                    .iter()
                    .map(|coherence| coherence.occurrence.len())
                    .chain(
                        configuration
                            .frame
                            .iter()
                            .map(|frame| frame.rule.len() + frame.held.len()),
                    )
                    .sum::<usize>()
            })
            .sum::<usize>();
        occurrence + self.event.len()
    }

    pub fn name(&self) -> String {
        format!("x{}", self.key)
    }

    pub fn extent(&self) -> Extent {
        Extent {
            mode: self.mode,
            closed: self.closed,
        }
    }

    pub fn inferred(&self, event: usize) -> bool {
        !self.event[event].deduction.is_empty()
    }
}
