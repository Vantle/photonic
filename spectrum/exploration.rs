use crate::budget::Budget;
use crate::configuration::{Coherence, Configuration, Frame, Occurrence, Opener, Value};
use crate::numbering::Numbering;
use crate::order::{self, Canonical, Naming};
use crate::recording::{Engine, Mode, Order};
use frontend::source::{Definition, Program};
use photonic::laser::Laser;
use photonic::place::Place;
use photonic::prism::{Outcome, Reach, Verdict};
use photonic::runtime::Runtime;
use photonic::snapshot::{self, Link, Node};
use photonic::status::Status;
use std::collections::VecDeque;

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

pub struct Plan {
    pub canonical: Canonical,
    pub mode: Mode,
    pub engine: Engine,
    pub budget: Budget,
    pub goal: Option<Program>,
    pub key: String,
}

struct Record {
    closed: bool,
    reached: bool,
    work: usize,
    rule: Vec<Rule>,
    configuration: Vec<Configuration>,
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
fn show(configuration: Configuration, naming: &Naming) -> Configuration {
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

fn configuration(node: &Node) -> Configuration {
    let token = occurrence;
    Configuration {
        coherence: node
            .world
            .iter()
            .map(|world| Coherence {
                frame: world.frame,
                occurrence: world.particle.iter().map(token).collect(),
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
                rule: frame.particle.iter().map(token).collect(),
                held: frame.held.iter().map(token).collect(),
            })
            .collect(),
        supported: node.status == Status::Supported,
    }
}

fn rule(definition: &snapshot::Definition, naming: &Naming) -> Rule {
    let plain = naming.show(&definition.rule);
    Rule {
        text: frontend::text::definition(&plain),
        canonical: plain.canonical(),
        definition: plain,
        scope: None,
    }
}

fn key(
    canonical: &Canonical,
    mode: Mode,
    engine: Engine,
    budget: Budget,
    goal: Option<&Program>,
) -> String {
    let program = &canonical.program;
    let goal = goal.map(|goal| (&goal.initial, &goal.rule));
    format!(
        "{:016x}",
        hashing::value(&(
            &program.initial,
            &program.rule,
            &canonical.naming,
            mode,
            engine,
            budget,
            goal
        ))
    )
}

impl Plan {
    pub fn new(
        program: &Program,
        mode: Mode,
        engine: Engine,
        budget: Budget,
        goal: Option<Program>,
    ) -> Self {
        let canonical = match mode {
            Mode::Exhaustive | Mode::Plain => order::exhaustive(program),
            Mode::Path => order::source(program),
        };
        let engine = if mode == Mode::Plain {
            Engine::Laser
        } else {
            engine
        };
        let goal = goal.map(|goal| {
            let mut hidden = canonical.naming.hide(&goal);
            hidden.rule.sort();
            hidden
        });
        let key = key(&canonical, mode, engine, budget, goal.as_ref());
        Self {
            canonical,
            mode,
            engine,
            budget,
            goal,
            key,
        }
    }
}

impl Exploration {
    pub fn new(plan: Plan) -> Self {
        match (plan.mode, plan.engine) {
            (Mode::Exhaustive, Engine::Interpreter) => Self::interpret(plan),
            (Mode::Exhaustive, Engine::Laser) | (Mode::Plain, _) => Self::compile(plan),
            (Mode::Path, _) => Self::walk(plan),
        }
    }

    fn interpret(plan: Plan) -> Self {
        let naming = &plan.canonical.naming;
        let mut runtime = Runtime::new(&plan.canonical.program);
        runtime.run(plan.budget.work, plan.budget.limit());
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
            reached: false,
            work: snapshot.work,
            rule: snapshot
                .definition
                .iter()
                .map(|entry| self::rule(entry, naming))
                .collect(),
            configuration: snapshot.state.iter().map(self::configuration).collect(),
            event,
        };
        let explorer = Explorer::Interpreter {
            reach: runtime.reach(),
            resource,
        };
        Self::assemble(plan, record, explorer)
    }

    fn compile(plan: Plan) -> Self {
        let naming = &plan.canonical.naming;
        let mut laser = match plan.mode {
            Mode::Plain => Laser::plain(&plan.canonical.program),
            Mode::Exhaustive | Mode::Path => Laser::new(&plan.canonical.program),
        };
        laser.run(plan.budget.work, plan.budget.limit());
        let report = laser.report();
        let record = Record {
            closed: report.closed,
            reached: false,
            work: report.work,
            rule: report
                .definition
                .iter()
                .map(|entry| self::rule(entry, naming))
                .collect(),
            configuration: report.state.iter().map(self::configuration).collect(),
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

    fn walk(mut plan: Plan) -> Self {
        let naming = &plan.canonical.naming;
        let mut search =
            photonic::path::Search::new(plan.canonical.program.clone(), plan.goal.take());
        search.run(plan.budget.work, plan.budget.limit());
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
        let reached = report.outcome == Outcome::Reached;
        let record = Record {
            closed: reached,
            reached,
            work: report.work,
            rule: report
                .definition
                .iter()
                .map(|entry| self::rule(entry, naming))
                .collect(),
            configuration: report.state.iter().map(self::configuration).collect(),
            event,
        };
        Self::assemble(plan, record, Explorer::Path)
    }

    fn assemble(plan: Plan, mut record: Record, explorer: Explorer) -> Self {
        let numbering = (plan.mode != Mode::Path).then(|| {
            let numbering = Numbering::new(&record.configuration, &record.event);
            let (configuration, event) = numbering.apply(
                std::mem::take(&mut record.configuration),
                std::mem::take(&mut record.event),
            );
            record.configuration = configuration;
            record.event = event;
            numbering
        });
        record.configuration = std::mem::take(&mut record.configuration)
            .into_iter()
            .map(|configuration| show(configuration, &plan.canonical.naming))
            .collect();
        let count = record.configuration.len();
        let mut outgoing = vec![Vec::new(); count];
        let mut incoming = vec![Vec::new(); count];
        for (index, event) in record.event.iter().enumerate() {
            outgoing[event.source].push(index);
            incoming[event.target].push(index);
        }
        let mut parent = vec![None; count];
        let mut depth = vec![None; count];
        if count > 0 {
            depth[0] = Some(0);
        }
        let mut queue = VecDeque::from([0]);
        while let Some(current) = queue.pop_front() {
            let Some(level) = depth.get(current).copied().flatten() else {
                continue;
            };
            for &index in &outgoing[current] {
                let event = &record.event[index];
                if !event.supported || depth[event.target].is_some() {
                    continue;
                }
                depth[event.target] = Some(level + 1);
                parent[event.target] = Some(index);
                queue.push_back(event.target);
            }
        }
        let mut rule = record.rule;
        for frame in record
            .configuration
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
            mode: plan.mode,
            engine: plan.engine,
            order: plan.canonical.order,
            shape: plan.canonical.shape,
            naming: plan.canonical.naming,
            program: plan.canonical.program,
            closed: record.closed,
            reached: record.reached,
            work: record.work,
            rule,
            configuration: record.configuration,
            event: record.event,
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
        let mut result = Vec::new();
        let mut cursor = configuration;
        while let Some(event) = self.parent[cursor] {
            result.push(event);
            cursor = self.event[event].source;
        }
        result.reverse();
        Some(result)
    }

    pub fn verdict(&self, target: &Program, preserve: bool) -> Option<Verdict> {
        let mut hidden = self.naming.hide(target);
        if preserve {
            hidden.preserve(&self.program);
        }
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

    pub fn settled(&self) -> bool {
        self.closed && self.mode != Mode::Path
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

    pub fn inferred(&self, event: usize) -> bool {
        !self.event[event].deduction.is_empty()
    }
}
