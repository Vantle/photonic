use crate::budget::Budget;
use crate::configuration::Configuration;
use crate::exploration::{self, Plan, Rule};
use crate::failure::{Code, Failure};
use crate::order::Naming;
use crate::recording::Order;
use frontend::source::Program;
use photonic::laser::makeup::Makeup;
use photonic::laser::net::{self, Cycle, Net};
use photonic::stop::Stop;
use schemars::JsonSchema;
use serde::Serialize;
use std::sync::OnceLock;

// Every schedule of plain events of a program, explored through its net of parts on the GPU through
// Metal where there is one and on the host otherwise: counts, the configurations where runs end,
// whether a run can go on forever, why it stopped short of closing and the work grounding took,
// one step for each match it read. It keeps no events, so it names nothing by a handle.
pub struct Survey {
    pub(crate) key: String,
    pub(crate) order: Order,
    pub(crate) shape: Option<u64>,
    pub(crate) closed: bool,
    pub(crate) stop: Vec<Stop>,
    pub(crate) device: Device,
    pub(crate) configuration: usize,
    pub(crate) event: u64,
    pub(crate) endless: Option<bool>,
    pub(crate) work: usize,
    pub(crate) rule: Vec<Rule>,
    pub(crate) end: Vec<Configuration>,
    naming: Naming,
    program: Program,
    marking: Vec<Makeup>,
    net: Net,
}

fn failure(message: impl ToString) -> Failure {
    Failure::new(Code::Engine, message.to_string())
}

// Where metal explored: on the GPU, or on the host's net where there is no GPU with Metal 3, with
// the configurations each keeps unless a question keeps fewer.
#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
#[schemars(
    description = "Where metal explored: the GPU, or the host's net where there is no GPU with Metal 3, and the configurations it keeps unless a question keeps fewer."
)]
pub(crate) enum Device {
    Gpu { capacity: usize },
    Host { capacity: usize },
}

impl Device {
    pub(crate) fn capacity(self) -> usize {
        match self {
            Self::Gpu { capacity } | Self::Host { capacity } => capacity,
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Gpu { .. } => "the GPU",
            Self::Host { .. } => "the host",
        }
    }
}

// The GPU's engine, its kernels compiled once for every survey, or none where there is no GPU.
fn engine() -> Result<Option<&'static wave::engine::Engine>, Failure> {
    static ENGINE: OnceLock<Result<Option<wave::engine::Engine>, String>> = OnceLock::new();
    match ENGINE.get_or_init(|| wave::engine::Engine::new().map_err(|error| error.to_string())) {
        Ok(engine) => Ok(engine.as_ref()),
        Err(message) => Err(failure(message)),
    }
}

// Where metal explores and the configurations it keeps unless a question keeps fewer: as many as
// the GPU holds, or HOST where the host explores alone, one thread finding about a million a
// second.
pub(crate) fn device() -> Result<Device, Failure> {
    Ok(match engine()? {
        Some(engine) => Device::Gpu {
            capacity: engine.capacity(),
        },
        None => Device::Host { capacity: HOST },
    })
}

const HOST: usize = 1 << 20;

// A net meets a configuration whose root ties to one of its components at the start or while it
// explores, on the host or the GPU, and each time it has no net of parts for metal.
fn unsupported(reason: net::Unsupported) -> Failure {
    failure(format!(
        "{reason}, so the program has no net of parts for metal; explore it with laser"
    ))
}

fn explore(net: &mut Net, budget: &Budget) -> Result<net::Exploration, Failure> {
    let limit = budget.limit();
    match engine()? {
        Some(engine) => engine
            .explore(net, budget.work, limit, Cycle::Find)
            .map_err(|error| match error {
                wave::failure::Failure::Unsupported(reason) => unsupported(reason),
                error => failure(error),
            }),
        None => net
            .explore(budget.work, limit, Cycle::Find)
            .map_err(unsupported),
    }
}

impl Survey {
    pub(crate) fn new(plan: Plan) -> Result<Self, Failure> {
        let program = plan.canonical.program;
        let mut net = Net::new(&program).map_err(unsupported)?;
        let explored = explore(&mut net, &plan.budget)?;
        let device = device()?;
        let naming = plan.canonical.naming;
        let rule = net
            .definition()
            .iter()
            .map(|definition| exploration::rule(definition, &naming))
            .collect();
        let end = explored
            .end
            .iter()
            .map(|marking| {
                exploration::show(exploration::configuration(&net.node(marking)), &naming)
            })
            .collect();
        Ok(Self {
            key: plan.key,
            order: plan.canonical.order,
            shape: plan.canonical.shape,
            closed: explored.closed,
            stop: explored.stop,
            device,
            configuration: explored.configuration,
            event: explored.event,
            endless: explored.endless,
            work: explored.work,
            rule,
            end,
            naming,
            program,
            marking: explored.end,
            net,
        })
    }

    // Which ends are the complete configuration of a target, as Prism reads it.
    pub(crate) fn target(&self, target: &Program, preserve: bool) -> Vec<bool> {
        let hidden = exploration::hide(&self.naming, &self.program, target, preserve);
        let found = self.net.find(&hidden);
        self.marking
            .iter()
            .map(|marking| found.as_ref() == Some(marking))
            .collect()
    }
}
