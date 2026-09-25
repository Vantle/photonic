use crate::cause::Role;
use crate::context::Context;
use crate::exploration::{self, Exploration};
use crate::failure::Failure;
use crate::handle::Handle;
use crate::lineage;
use crate::recording::{Mode, Recording};
use crate::render;
use photonic::place::Place;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(
    description = "Describe one thing in an exploration by its handle: a rule, configuration, coherence, occurrence, scope frame or event."
)]
pub struct Request {
    #[serde(flatten)]
    pub recording: Recording,
    #[schemars(description = "A handle such as r2, s11, e12, s11.c0, s11.o1 or s10.f1.")]
    pub handle: String,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Item {
    pub handle: String,
    pub text: String,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Move {
    pub event: String,
    pub rule: String,
    pub configuration: String,
    pub text: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub inferred: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub unsupported: bool,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Part {
    pub handle: String,
    pub frame: String,
    pub text: String,
    pub occurrence: Vec<Item>,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Scope {
    pub handle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opener: Option<String>,
    pub rule: Vec<Item>,
    pub held: Vec<Item>,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum View {
    Rule {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        scope: Option<String>,
        fired: usize,
        inferred: usize,
        event: Vec<Move>,
    },
    Configuration {
        text: String,
        supported: bool,
        #[schemars(
            description = "No event can happen here, which a closed exhaustive exploration proves."
        )]
        end: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        depth: Option<usize>,
        coherence: Vec<Part>,
        frame: Vec<Scope>,
        next: Vec<Move>,
        previous: Vec<Move>,
    },
    Coherence {
        configuration: String,
        part: Part,
    },
    Occurrence {
        configuration: String,
        text: String,
        place: Vec<String>,
    },
    Frame {
        configuration: String,
        scope: Scope,
        coherence: Vec<String>,
    },
    Event {
        rule: String,
        source: String,
        target: String,
        supported: bool,
        deduction: Vec<String>,
        exact: Vec<Item>,
        witness: Vec<Item>,
        read: Vec<Item>,
        produced: Vec<Item>,
    },
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Answer {
    pub exploration: String,
    pub handle: String,
    #[serde(flatten)]
    pub view: View,
}

const EVENT: usize = 8;

fn frame(configuration: usize, index: usize) -> String {
    if index == 0 {
        return "root".to_owned();
    }
    Handle::Frame(configuration, index).to_string()
}

fn part(exploration: &Exploration, configuration: usize, world: usize) -> Part {
    let coherence = &exploration.configuration[configuration].coherence[world];
    Part {
        handle: Handle::Coherence(configuration, world).to_string(),
        frame: frame(configuration, coherence.frame),
        text: render::coherence(exploration, coherence),
        occurrence: coherence
            .occurrence
            .iter()
            .map(|occurrence| render::item(exploration, configuration, occurrence.id))
            .collect(),
    }
}

fn scope(exploration: &Exploration, configuration: usize, index: usize) -> Scope {
    let entry = &exploration.configuration[configuration].frame[index];
    let item = |occurrence: &crate::exploration::Occurrence| {
        render::item(exploration, configuration, occurrence.id)
    };
    Scope {
        handle: frame(configuration, index),
        opener: entry.opener.map(|rule| Handle::Rule(rule).to_string()),
        rule: entry.rule.iter().map(item).collect(),
        held: entry.held.iter().map(item).collect(),
    }
}

fn produced(exploration: &Exploration, event: usize) -> Vec<Item> {
    let target = exploration.event[event].target;
    exploration.configuration[target]
        .coherence
        .iter()
        .enumerate()
        .flat_map(|(world, coherence)| {
            coherence
                .occurrence
                .iter()
                .map(move |occurrence| Place::World(world, occurrence.id))
        })
        .filter(|&place| lineage::origin(exploration, event, place).0 == Role::Produced)
        .map(|place| render::item(exploration, target, exploration::id(place)))
        .collect()
}

pub(crate) fn event(exploration: &Exploration, index: usize) -> View {
    let entry = &exploration.event[index];
    let item = |place: &Place| render::place(exploration, entry.source, *place);
    View::Event {
        rule: format!(
            "{} {}",
            Handle::Rule(entry.rule),
            exploration.rule[entry.rule].text
        ),
        source: Handle::Configuration(entry.source).to_string(),
        target: Handle::Configuration(entry.target).to_string(),
        supported: entry.supported,
        deduction: entry
            .deduction
            .iter()
            .map(|&event| Handle::Event(event).to_string())
            .collect(),
        exact: entry.exact.iter().map(item).collect(),
        witness: entry
            .footprint
            .iter()
            .filter(|place| !entry.exact.contains(place))
            .map(item)
            .collect(),
        read: entry.read.iter().map(item).collect(),
        produced: if exploration.mode == Mode::Path {
            Vec::new()
        } else {
            produced(exploration, index)
        },
    }
}

fn view(exploration: &Exploration, handle: Handle) -> View {
    match handle {
        Handle::Rule(index) => {
            let fired = exploration.firing(index).collect::<Vec<_>>();
            View::Rule {
                text: exploration.rule[index].text.clone(),
                scope: exploration.rule[index]
                    .scope
                    .map(|scope| Handle::Rule(scope).to_string()),
                fired: fired.len(),
                inferred: fired
                    .iter()
                    .filter(|&&event| exploration.inferred(event))
                    .count(),
                event: (0..exploration.event.len())
                    .filter(|&event| exploration.event[event].rule == index)
                    .take(EVENT)
                    .map(|event| render::movement(exploration, event, true))
                    .collect(),
            }
        }
        Handle::Configuration(index) => {
            let entry = &exploration.configuration[index];
            View::Configuration {
                text: render::configuration(exploration, index),
                supported: entry.supported,
                end: exploration.settled() && exploration.stuck(index),
                depth: exploration.depth[index],
                coherence: (0..entry.coherence.len())
                    .map(|world| part(exploration, index, world))
                    .collect(),
                frame: (0..entry.frame.len())
                    .map(|frame| scope(exploration, index, frame))
                    .collect(),
                next: exploration.outgoing[index]
                    .iter()
                    .map(|&event| render::movement(exploration, event, true))
                    .collect(),
                previous: exploration.incoming[index]
                    .iter()
                    .map(|&event| render::movement(exploration, event, false))
                    .collect(),
            }
        }
        Handle::Coherence(index, world) => View::Coherence {
            configuration: Handle::Configuration(index).to_string(),
            part: part(exploration, index, world),
        },
        Handle::Occurrence(index, id) => {
            let entry = &exploration.configuration[index];
            let world = entry
                .coherence
                .iter()
                .enumerate()
                .filter(|(_, coherence)| {
                    coherence
                        .occurrence
                        .iter()
                        .any(|occurrence| occurrence.id == id)
                })
                .map(|(world, _)| Handle::Coherence(index, world).to_string());
            let frame = entry
                .frame
                .iter()
                .enumerate()
                .filter(|(_, frame)| {
                    frame
                        .rule
                        .iter()
                        .chain(&frame.held)
                        .any(|occurrence| occurrence.id == id)
                })
                .map(|(position, _)| self::frame(index, position));
            View::Occurrence {
                configuration: Handle::Configuration(index).to_string(),
                text: render::item(exploration, index, id).text,
                place: world.chain(frame).collect(),
            }
        }
        Handle::Frame(index, position) => View::Frame {
            configuration: Handle::Configuration(index).to_string(),
            scope: scope(exploration, index, position),
            coherence: exploration.configuration[index]
                .coherence
                .iter()
                .enumerate()
                .filter(|(_, coherence)| coherence.frame == position)
                .map(|(world, _)| Handle::Coherence(index, world).to_string())
                .collect(),
        },
        Handle::Event(index) => event(exploration, index),
    }
}

pub fn answer(request: &Request, context: &mut Context<'_>) -> Result<Answer, Failure> {
    let exploration = context.exploration(&request.recording)?;
    let handle = request.handle.parse::<Handle>()?.check(&exploration)?;
    Ok(Answer {
        exploration: exploration.name(),
        handle: handle.to_string(),
        view: view(&exploration, handle),
    })
}

fn list(item: &[Item]) -> String {
    if item.is_empty() {
        return "none".to_owned();
    }
    item.iter()
        .map(|item| format!("{} {}", item.handle, item.text))
        .collect::<Vec<_>>()
        .join(" · ")
}

impl Answer {
    pub fn text(&self) -> String {
        let mut line = Vec::new();
        match &self.view {
            View::Rule {
                text,
                scope,
                fired,
                inferred,
                event,
            } => {
                let place = scope
                    .as_ref()
                    .map(|scope| format!(" · local to the scope {scope} opens"))
                    .unwrap_or_default();
                let count = match (*fired, *inferred) {
                    (0, _) => "never fires".to_owned(),
                    (fired, 0) => format!("fires {}", render::count(fired, "time")),
                    (fired, inferred) => {
                        format!(
                            "fires {}, {inferred} inferred",
                            render::count(fired, "time")
                        )
                    }
                };
                line.push(format!("{} {text}{place} · {count}", self.handle));
                render::table("event", event, &mut line);
            }
            View::Configuration {
                text,
                supported,
                end,
                depth,
                coherence,
                frame,
                next,
                previous,
            } => {
                let support = if *supported { "" } else { " · unsupported" };
                let depth = depth
                    .map(|depth| format!(" · depth {depth}"))
                    .unwrap_or_default();
                line.push(format!("{} {text}{support}{depth}", self.handle));
                for (position, part) in coherence.iter().enumerate() {
                    line.push(format!(
                        "{}{:<8} {:<6} {}   {}",
                        render::row("coherence", position == 0),
                        part.handle,
                        part.frame,
                        part.text,
                        list(&part.occurrence)
                    ));
                }
                for (position, scope) in frame.iter().enumerate().skip(1) {
                    let opener = scope
                        .opener
                        .as_ref()
                        .map(|rule| format!("opened by {rule}"))
                        .unwrap_or_default();
                    line.push(format!(
                        "{}{:<8} {opener}   rules {}   holds {}",
                        render::row("scope", position == 1),
                        scope.handle,
                        list(&scope.rule),
                        list(&scope.held)
                    ));
                }
                render::table("next", next, &mut line);
                if *end {
                    line.push(format!(
                        "{}none: an end configuration",
                        render::row("next", true)
                    ));
                }
                render::table("previous", previous, &mut line);
            }
            View::Coherence {
                configuration,
                part,
            } => {
                line.push(format!(
                    "{} in {configuration} · {} · {}",
                    self.handle, part.frame, part.text
                ));
                line.push(format!(
                    "{}{}",
                    render::row("occurrence", true),
                    list(&part.occurrence)
                ));
            }
            View::Occurrence {
                configuration,
                text,
                place,
            } => {
                line.push(format!(
                    "{} {text} in {configuration} · {}",
                    self.handle,
                    place.join(" · ")
                ));
            }
            View::Frame {
                configuration,
                scope,
                coherence,
            } => {
                let opener = scope
                    .opener
                    .as_ref()
                    .map(|rule| format!(" · opened by {rule}"))
                    .unwrap_or_default();
                line.push(format!("{} in {configuration}{opener}", self.handle));
                line.push(format!(
                    "{}{}",
                    render::row("rules", true),
                    list(&scope.rule)
                ));
                line.push(format!(
                    "{}{}",
                    render::row("holds", true),
                    list(&scope.held)
                ));
                line.push(format!(
                    "{}{}",
                    render::row("coherence", true),
                    if coherence.is_empty() {
                        "none".to_owned()
                    } else {
                        coherence.join(" · ")
                    }
                ));
            }
            View::Event {
                rule,
                source,
                target,
                supported,
                deduction,
                exact,
                witness,
                read,
                produced,
            } => {
                let support = if *supported { "" } else { " · unsupported" };
                line.push(format!(
                    "{} {rule} · {source} → {target}{support}",
                    self.handle
                ));
                if !deduction.is_empty() {
                    line.push(format!(
                        "{}inferred by {}",
                        render::row("deduction", true),
                        deduction.join(" ")
                    ));
                }
                line.push(format!("{}{}", render::row("exact", true), list(exact)));
                if !witness.is_empty() {
                    line.push(format!("{}{}", render::row("witness", true), list(witness)));
                }
                if !read.is_empty() {
                    line.push(format!("{}{}", render::row("reads", true), list(read)));
                }
                if !produced.is_empty() {
                    line.push(format!(
                        "{}{}",
                        render::row("produces", true),
                        list(produced)
                    ));
                }
            }
        }
        line.join("\n")
    }
}
