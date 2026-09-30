use crate::context::Context;
use crate::failure::{Code, Failure};
use crate::handle::Handle;
use crate::inspect::Move;
use crate::recording::{Mode, Recording};
use crate::render::{self, Extent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(
    description = "List the agenda at a configuration: every event that can happen there and where it leads. Stepping is choosing one."
)]
pub struct Request {
    #[serde(flatten)]
    pub recording: Recording,
    #[serde(default = "start")]
    #[schemars(description = "The configuration to step from; s0 is the start.")]
    pub handle: String,
}

fn start() -> String {
    "s0".to_owned()
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Answer {
    pub(crate) exploration: String,
    pub(crate) mode: Mode,
    #[schemars(
        description = "Whether the exploration closed: it explored every future, or in plain mode every plain schedule, within its budget, so what it lacks is absent. A direct path follows one run and never closes."
    )]
    pub(crate) closed: bool,
    pub(crate) handle: String,
    pub(crate) text: String,
    pub(crate) agenda: Vec<Move>,
}

pub(crate) fn answer(request: &Request, context: &mut Context<'_>) -> Result<Answer, Failure> {
    let exploration = context.exploration(&request.recording)?;
    let handle = request.handle.parse::<Handle>()?.check(&exploration)?;
    let Handle::Configuration(index) = handle else {
        return Err(Failure::new(
            Code::Handle,
            "step starts from a configuration, such as s0",
        ));
    };
    Ok(Answer {
        exploration: exploration.name(),
        mode: exploration.mode,
        closed: exploration.closed,
        handle: handle.to_string(),
        text: render::configuration(&exploration, index),
        agenda: exploration.outgoing[index]
            .iter()
            .map(|&event| render::movement(&exploration, event, true))
            .collect(),
    })
}

impl Answer {
    pub(crate) fn text(&self) -> String {
        let mut line = vec![format!("{} {}", self.handle, self.text)];
        let extent = Extent::new(self.mode, self.closed);
        let (label, empty) = match extent {
            Extent::Path => ("taken", "the path stops here"),
            Extent::Closed => ("agenda", "no event can happen here: an end configuration"),
            Extent::Open => ("agenda", "no event is recorded here"),
        };
        if self.agenda.is_empty() {
            line.push(empty.to_owned());
        }
        render::table(label, &self.agenda, &mut line);
        if extent == Extent::Open {
            line.push("the exploration is open; more events may happen here".to_owned());
        }
        line.join("\n")
    }
}
