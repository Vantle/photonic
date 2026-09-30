use crate::context::Context;
use crate::extent::{Extent, Kind};
use crate::failure::{Code, Failure};
use crate::handle::Handle;
use crate::inspect::Move;
use crate::recording::Recording;
use crate::render;
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
    #[serde(flatten)]
    pub(crate) extent: Extent,
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
        extent: exploration.extent(),
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
        let extent = self.extent;
        let (label, empty) = match extent.kind() {
            Kind::Path => ("taken", "the path stops here"),
            Kind::Closed => ("agenda", "no event can happen here: an end configuration"),
            Kind::Open => ("agenda", "no event is recorded here"),
        };
        if self.agenda.is_empty() {
            line.push(empty.to_owned());
        }
        render::table(label, &self.agenda, &mut line);
        line.extend(extent.partial().map(str::to_owned));
        line.join("\n")
    }
}
