use crate::context::Context;
use crate::exploration::Exploration;
use crate::failure::{Code, Failure};
use crate::handle::Handle;
use crate::inspect::{self, Move};
use crate::lineage;
use crate::recording::Recording;
use crate::render;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(
    description = "Explain why something is here: the shortest grounded path to a configuration, the match and deduction of an event, or the lineage of an occurrence."
)]
pub struct Request {
    #[serde(flatten)]
    pub recording: Recording,
    #[schemars(
        description = "A configuration, event or occurrence handle, such as s11, e12 or s11.o1."
    )]
    pub handle: String,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[schemars(
    description = "How an occurrence reached the next configuration: initial in the start, produced by a rule, passed into a scope as witness, held by a scope, left in the remainder, or untouched."
)]
pub(crate) enum Role {
    Initial,
    Produced,
    Witness,
    Held,
    Remainder,
    Untouched,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Step {
    pub(crate) configuration: String,
    pub(crate) occurrence: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) event: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rule: Option<String>,
    pub(crate) role: Role,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) source: Vec<String>,
    pub(crate) text: String,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub(crate) enum Reason {
    Configuration {
        text: String,
        supported: bool,
        path: Vec<Move>,
    },
    Event {
        event: inspect::View,
        deduction: Vec<Move>,
    },
    Occurrence {
        text: String,
        configuration: String,
        lineage: Vec<Step>,
    },
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Answer {
    pub(crate) exploration: String,
    pub(crate) handle: String,
    #[serde(flatten)]
    pub(crate) reason: Reason,
}

fn path(exploration: &Exploration, configuration: usize) -> Vec<Move> {
    exploration
        .path(configuration)
        .unwrap_or_default()
        .iter()
        .map(|&event| render::movement(exploration, event, true))
        .collect()
}

fn phrase(
    exploration: &Exploration,
    configuration: usize,
    place: &[photonic::place::Place],
) -> String {
    let text = place
        .iter()
        .map(|&place| render::place(exploration, configuration, place).text)
        .collect::<Vec<_>>();
    if text.is_empty() {
        return "nothing".to_owned();
    }
    text.join(", ")
}

fn step(exploration: &Exploration, line: &lineage::Line, atom: &str) -> Step {
    let configuration = Handle::Configuration(line.configuration).to_string();
    let occurrence = Handle::Occurrence(line.configuration, line.occurrence).to_string();
    let Some(event) = line.event else {
        return Step {
            configuration,
            occurrence,
            event: None,
            rule: None,
            role: line.role,
            source: Vec::new(),
            text: format!(
                "initial in {}",
                render::configuration(exploration, line.configuration)
            ),
        };
    };
    let entry = &exploration.event[event];
    let source = line
        .source
        .iter()
        .map(|&place| render::place(exploration, entry.source, place))
        .collect::<Vec<_>>();
    let inferred = if entry.deduction.is_empty() {
        String::new()
    } else {
        format!(
            "inferred by {}; ",
            entry
                .deduction
                .iter()
                .map(|&event| Handle::Event(event).to_string())
                .collect::<Vec<_>>()
                .join(" ")
        )
    };
    let scope = exploration.configuration[entry.target].frame.len()
        > exploration.configuration[entry.source].frame.len();
    let text = match line.role {
        Role::Witness if scope => format!("{inferred}the scope receives {atom} as witness"),
        Role::Witness => format!("{inferred}{atom} is matched as witness"),
        Role::Held => format!("{inferred}the scope holds {atom}"),
        Role::Remainder => format!(
            "{inferred}consumes {}; {atom} stays in the remainder",
            phrase(exploration, entry.source, &entry.exact)
        ),
        Role::Untouched => format!("{inferred}{atom} passes untouched"),
        Role::Produced if source.is_empty() => format!("{inferred}produces {atom}"),
        Role::Produced => format!(
            "{inferred}produces {atom} from {}",
            source
                .iter()
                .map(|item| format!("{} {}", item.handle, item.text))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Role::Initial => format!("{inferred}initial"),
    };
    Step {
        configuration,
        occurrence,
        event: Some(Handle::Event(event).to_string()),
        rule: Some(format!(
            "{} {}",
            Handle::Rule(entry.rule),
            render::brief(exploration, entry.rule)
        )),
        role: line.role,
        source: source.into_iter().map(|item| item.handle).collect(),
        text,
    }
}

pub(crate) fn answer(request: &Request, context: &mut Context<'_>) -> Result<Answer, Failure> {
    let exploration = context.exploration(&request.recording)?;
    let handle = request.handle.parse::<Handle>()?.check(&exploration)?;
    let reason = match handle {
        Handle::Configuration(index) => Reason::Configuration {
            text: render::configuration(&exploration, index),
            supported: exploration.configuration[index].supported,
            path: path(&exploration, index),
        },
        Handle::Event(index) => Reason::Event {
            event: inspect::event(&exploration, index),
            deduction: exploration.event[index]
                .deduction
                .iter()
                .map(|&event| render::movement(&exploration, event, true))
                .collect(),
        },
        Handle::Occurrence(index, id) => {
            let atom = render::item(&exploration, index, id).text;
            let lineage = lineage::lineage(&exploration, index, id)?;
            Reason::Occurrence {
                configuration: render::configuration(&exploration, index),
                lineage: lineage
                    .iter()
                    .filter(|line| line.role != Role::Untouched)
                    .map(|line| step(&exploration, line, &atom))
                    .collect(),
                text: atom,
            }
        }
        _ => {
            return Err(Failure::new(
                Code::Handle,
                "cause explains a configuration, an event or an occurrence, such as s11, e12 or s11.o1",
            ));
        }
    };
    Ok(Answer {
        exploration: exploration.name(),
        handle: handle.to_string(),
        reason,
    })
}

impl Answer {
    pub(crate) fn text(&self) -> String {
        let mut line = Vec::new();
        match &self.reason {
            Reason::Configuration {
                text,
                supported,
                path,
            } => {
                let support = if *supported {
                    ""
                } else {
                    " · unsupported: no grounded path"
                };
                line.push(format!("{} {text}{support}", self.handle));
                if path.is_empty() && *supported {
                    line.push("the start".to_owned());
                }
                render::table("path", path, &mut line);
            }
            Reason::Event { event, deduction } => {
                line.push(
                    inspect::Answer {
                        exploration: self.exploration.clone(),
                        handle: self.handle.clone(),
                        view: event.clone(),
                    }
                    .text(),
                );
                render::table("deduction", deduction, &mut line);
            }
            Reason::Occurrence {
                text,
                configuration,
                lineage,
            } => {
                let owner = self.handle.split('.').next().unwrap_or_default();
                line.push(format!("{text} in {owner} = {configuration}"));
                let width = lineage
                    .iter()
                    .filter_map(|step| step.rule.as_ref())
                    .map(|rule| rule.chars().count())
                    .max()
                    .unwrap_or(0);
                for step in lineage.iter().rev() {
                    let label = step.event.as_deref().unwrap_or(&step.configuration);
                    let rule = step.rule.as_deref().unwrap_or("");
                    line.push(format!("  {label:<5} {rule:<width$}   {}", step.text));
                }
            }
        }
        line.join("\n")
    }
}
