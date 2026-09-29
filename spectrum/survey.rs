use crate::budget::Budget;
use crate::configuration::Configuration;
use crate::exploration::{self, Plan, Rule};
use crate::failure::{Code, Failure};
use crate::order::Naming;
use crate::recording::Order;
use frontend::source::Program;
use photonic::laser::net::{self, Cycle, Marking, Net};
use std::sync::OnceLock;

// Every schedule of plain events of a program, explored through its net of parts on the GPU through
// Metal where there is one and on the host otherwise: counts, the configurations where runs end,
// whether a run can go on forever and the work grounding took, one step for each match it read.
// It keeps no events, so it names nothing by a handle.
pub struct Survey {
    pub(crate) key: String,
    pub(crate) order: Order,
    pub(crate) shape: Option<u64>,
    pub(crate) closed: bool,
    pub(crate) configuration: usize,
    pub(crate) event: u64,
    pub(crate) endless: bool,
    pub(crate) work: usize,
    pub(crate) rule: Vec<Rule>,
    pub(crate) end: Vec<Configuration>,
    naming: Naming,
    program: Program,
    marking: Vec<Marking>,
    net: Net,
}

fn failure(message: impl ToString) -> Failure {
    Failure::new(Code::Engine, message.to_string())
}

// The GPU's engine, its kernels compiled once for every survey, or none where there is no GPU.
fn device() -> Result<Option<&'static wave::engine::Engine>, Failure> {
    static DEVICE: OnceLock<Result<Option<wave::engine::Engine>, String>> = OnceLock::new();
    match DEVICE.get_or_init(|| wave::engine::Engine::new().map_err(|error| error.to_string())) {
        Ok(engine) => Ok(engine.as_ref()),
        Err(message) => Err(failure(message)),
    }
}

fn explore(net: &mut Net, budget: &Budget) -> Result<net::Exploration, Failure> {
    let limit = budget.limit();
    match device()? {
        Some(engine) => engine
            .explore(net, budget.work, limit, Cycle::Find)
            .map_err(failure),
        None => net
            .explore(budget.work, limit, Cycle::Find)
            .map_err(failure),
    }
}

impl Survey {
    pub(crate) fn new(plan: Plan) -> Result<Self, Failure> {
        let unsupported = |unsupported: net::Unsupported| {
            failure(format!(
                "{unsupported}, so the program has no net of parts for metal; explore it with laser"
            ))
        };
        let program = plan.canonical.program;
        let mut net = Net::new(&program).map_err(unsupported)?;
        let explored = explore(&mut net, &plan.budget)?;
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
            configuration: explored.configuration,
            event: explored.event,
            endless: explored.endless.unwrap_or_default(),
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
