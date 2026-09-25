use crate::budget::Budget;
use crate::failure::{Code, Failure};
use crate::order::{self, Canonical, Naming};
use crate::recording::{Mode, Order};
use photonic::place::Place;
use photonic::prism::{self, Verdict};
use photonic::snapshot::{self, Node};
use photonic::source::{Definition, Program};
use photonic::status::Status;
use std::collections::VecDeque;
use std::sync::{Mutex, PoisonError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Value {
    Atom(String),
    Rule(usize),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Occurrence {
    pub id: usize,
    pub value: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Coherence {
    pub frame: usize,
    pub occurrence: Vec<Occurrence>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    pub opener: Option<usize>,
    pub lexical: Option<usize>,
    pub rule: Vec<Occurrence>,
    pub held: Vec<Occurrence>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Configuration {
    pub coherence: Vec<Coherence>,
    pub frame: Vec<Frame>,
    pub supported: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rule {
    pub text: String,
    pub definition: Definition,
    pub canonical: Definition,
    pub scope: Option<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Link {
    pub target: Place,
    pub source: Vec<Place>,
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
    pub resource: Vec<Link>,
}

pub struct Exploration {
    pub key: String,
    pub mode: Mode,
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
    search: Option<Mutex<prism::Search>>,
}

pub struct Plan {
    pub canonical: Canonical,
    pub mode: Mode,
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

fn opener<'scope>(scope: &str, lookup: impl Fn(&str) -> Option<&'scope [usize]>) -> Option<usize> {
    let (parent, _) = scope.rsplit_once('/')?;
    let (owner, position) = parent.rsplit_once('/')?;
    let position = position.parse().ok()?;
    if owner == "value" {
        return Some(position);
    }
    lookup(owner)?.get(position).copied()
}

fn occurrence(token: &snapshot::Token, naming: &Naming) -> Occurrence {
    let rule = matches!(token.kind, snapshot::Kind::Rule)
        .then(|| token.label.strip_prefix('§')?.parse().ok())
        .flatten();
    Occurrence {
        id: token.id,
        value: rule.map_or_else(
            || Value::Atom(naming.name(&token.label).to_owned()),
            Value::Rule,
        ),
    }
}

fn configuration<'scope>(
    node: &Node,
    naming: &Naming,
    lookup: impl Fn(&str) -> Option<&'scope [usize]> + Copy,
) -> Configuration {
    let token = |token: &snapshot::Token| occurrence(token, naming);
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
                opener: opener(&frame.scope, lookup),
                lexical: frame.lexical,
                rule: frame.particle.iter().map(token).collect(),
                held: frame.held.iter().map(token).collect(),
            })
            .collect(),
        supported: node.status == Status::Supported,
    }
}

fn rule(definition: &snapshot::Definition, naming: &Naming) -> Result<Rule, Failure> {
    let inner = definition
        .display
        .strip_prefix('⟨')
        .and_then(|text| text.strip_suffix('⟩'))
        .unwrap_or(&definition.display);
    let text = naming.show(inner);
    let program = photonic::lowering::parse(&text).map_err(|error| {
        Failure::new(
            Code::Exploration,
            format!("the rule {text} does not read back: {error}"),
        )
    })?;
    let [value] = program.rule.as_slice() else {
        return Err(Failure::new(
            Code::Exploration,
            format!("the rule {text} reads back as {} rules", program.rule.len()),
        ));
    };
    let plain = Definition {
        name: String::new(),
        ..value.clone()
    };
    Ok(Rule {
        text: photonic::text::definition(&plain),
        canonical: plain.canonical(),
        definition: plain,
        scope: None,
    })
}

fn deduction(view: &[snapshot::View], evidence: &[usize]) -> Vec<usize> {
    if evidence
        .iter()
        .any(|&index| view[index].source == view[index].target)
    {
        return Vec::new();
    }
    let Some(&first) = evidence.first() else {
        return Vec::new();
    };
    let mut chain = Vec::new();
    let mut cursor = view[first].origin;
    while let Some(origin) = cursor {
        chain.push(origin.event);
        cursor = view[origin.view].origin;
    }
    chain.reverse();
    chain
}

fn key(canonical: &Canonical, mode: Mode, budget: Budget, goal: Option<&Program>) -> String {
    let program = &canonical.program;
    let goal = goal.map(|goal| (&goal.initial, &goal.rule));
    format!(
        "{:016x}",
        code::hashing::value(&(
            &program.initial,
            &program.rule,
            &canonical.naming,
            mode,
            budget,
            goal
        ))
    )
}

impl Plan {
    pub fn new(program: &Program, mode: Mode, budget: Budget, goal: Option<Program>) -> Self {
        let canonical = match mode {
            Mode::Exhaustive => order::exhaustive(program),
            Mode::Path => order::source(program),
        };
        let goal = goal.map(|goal| {
            let mut hidden = canonical.naming.hide(&goal);
            hidden.rule.sort();
            hidden
        });
        let key = key(&canonical, mode, budget, goal.as_ref());
        Self {
            canonical,
            mode,
            budget,
            goal,
            key,
        }
    }
}

impl Exploration {
    pub fn new(plan: Plan) -> Result<Self, Failure> {
        match plan.mode {
            Mode::Exhaustive => Self::exhaustive(plan),
            Mode::Path => Self::walk(plan),
        }
    }

    fn exhaustive(plan: Plan) -> Result<Self, Failure> {
        let naming = &plan.canonical.naming;
        let mut search = prism::Search::new(plan.canonical.program.clone(), Program::default());
        search.run(plan.budget.work, Some(plan.budget.limit()));
        let snapshot = search.snapshot();
        let event = snapshot
            .event
            .iter()
            .map(|event| {
                let rule = search.rule(event.id).ok_or_else(|| {
                    Failure::new(Code::Exploration, format!("event {} has no rule", event.id))
                })?;
                Ok(Event {
                    source: event.source,
                    target: event.target,
                    rule,
                    supported: event.status == Status::Supported,
                    footprint: event.footprint.clone(),
                    exact: event.exact.clone(),
                    read: event.read.clone(),
                    world: event.world.clone(),
                    deduction: deduction(&snapshot.view, &event.evidence),
                    resource: search
                        .resource(event.id)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|link| Link {
                            target: link.target,
                            source: link.source,
                        })
                        .collect(),
                })
            })
            .collect::<Result<Vec<_>, Failure>>()?;
        let record = Record {
            closed: snapshot.closed,
            reached: false,
            work: snapshot.work,
            rule: snapshot
                .definition
                .iter()
                .map(|entry| self::rule(entry, naming))
                .collect::<Result<Vec<_>, _>>()?,
            configuration: snapshot
                .state
                .iter()
                .map(|node| self::configuration(node, naming, |name| search.scope(name)))
                .collect(),
            event,
        };
        Ok(Self::assemble(plan, record, Some(search)))
    }

    fn walk(mut plan: Plan) -> Result<Self, Failure> {
        let naming = &plan.canonical.naming;
        let mut search = photonic::path::Search::new(
            plan.canonical.program.clone(),
            plan.goal.take().unwrap_or_default(),
        );
        search.run(plan.budget.work, plan.budget.limit());
        let report = search.report();
        let event = report
            .event
            .iter()
            .enumerate()
            .map(|(index, event)| {
                let rule = search.rule(index).ok_or_else(|| {
                    Failure::new(Code::Exploration, format!("step {index} has no rule"))
                })?;
                Ok(Event {
                    source: event.source,
                    target: event.target,
                    rule,
                    supported: true,
                    footprint: event.footprint.clone(),
                    exact: event.exact.clone(),
                    read: event.read.clone(),
                    world: Vec::new(),
                    deduction: Vec::new(),
                    resource: Vec::new(),
                })
            })
            .collect::<Result<Vec<_>, Failure>>()?;
        let reached = report.outcome == prism::Outcome::Reached;
        let record = Record {
            closed: reached,
            reached,
            work: report.work,
            rule: report
                .definition
                .iter()
                .map(|entry| self::rule(entry, naming))
                .collect::<Result<Vec<_>, _>>()?,
            configuration: report
                .state
                .iter()
                .map(|node| self::configuration(node, naming, |name| search.scope(name)))
                .collect(),
            event,
        };
        Ok(Self::assemble(plan, record, None))
    }

    fn assemble(plan: Plan, record: Record, search: Option<prism::Search>) -> Self {
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
            search: search.map(Mutex::new),
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
        let search = self.search.as_ref()?;
        let mut hidden = self.naming.hide(target);
        if preserve {
            hidden.preserve(&self.program);
        }
        let mut search = search.lock().unwrap_or_else(PoisonError::into_inner);
        search.target(hidden);
        Some(search.verdict())
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
        self.closed && self.mode == Mode::Exhaustive
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

    pub fn name(&self) -> String {
        format!("x{}", self.key)
    }

    pub fn inferred(&self, event: usize) -> bool {
        !self.event[event].deduction.is_empty()
    }
}
