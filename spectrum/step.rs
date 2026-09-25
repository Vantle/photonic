use crate::context::Context;
use crate::failure::{Code, Failure};
use crate::handle::Handle;
use crate::inspect::Move;
use crate::recording::{Mode, Recording};
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
    pub exploration: String,
    pub mode: Mode,
    pub complete: bool,
    pub handle: String,
    pub text: String,
    pub agenda: Vec<Move>,
}

pub fn answer(request: &Request, context: &mut Context<'_>) -> Result<Answer, Failure> {
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
        complete: exploration.closed,
        handle: handle.to_string(),
        text: render::configuration(&exploration, index),
        agenda: exploration.outgoing[index]
            .iter()
            .map(|&event| render::movement(&exploration, event, true))
            .collect(),
    })
}

impl Answer {
    pub fn text(&self) -> String {
        let mut line = vec![format!("{} {}", self.handle, self.text)];
        let (label, empty) = match (self.mode, self.complete) {
            (Mode::Path, _) => ("taken", "the path stops here"),
            (Mode::Exhaustive, true) => {
                ("agenda", "no event can happen here: an end configuration")
            }
            (Mode::Exhaustive, false) => (
                "agenda",
                "no event is recorded here; the exploration is open, so one may still happen",
            ),
        };
        if self.agenda.is_empty() {
            line.push(empty.to_owned());
        }
        render::table(label, &self.agenda, &mut line);
        line.join("\n")
    }
}
