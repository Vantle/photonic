use crate::configuration::Opener;
use crate::context::Context;
use crate::exploration::Exploration;
use crate::explored::Explored;
use crate::extent::{Extent, Kind};
use crate::failure::Failure;
use crate::handle::Handle;
use crate::recording::{Engine, Mode, Order, Recording};
use crate::render;
use crate::survey::Device;
use photonic::stop::{Bound, Stop};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const LIMIT: usize = 12;

fn limit() -> usize {
    LIMIT
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(
    description = "Explore every future of a program and summarize it: counts, end configurations and rule activity; on metal, counts and end configurations alone."
)]
pub struct Request {
    #[serde(flatten)]
    pub recording: Recording,
    #[serde(default = "limit")]
    #[schemars(description = "End configurations listed; the rest are counted.")]
    pub limit: usize,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct End {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "The configuration's handle; metal numbers nothing, so its ends have none."
    )]
    pub(crate) handle: Option<String>,
    pub(crate) scope: bool,
    pub(crate) text: String,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Activity {
    pub(crate) handle: String,
    pub(crate) text: String,
    pub(crate) fired: usize,
    pub(crate) inferred: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) first: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        with = "Option<String>",
        description = "The rule whose scope holds this rule, or program when the program opens that scope at the start."
    )]
    pub(crate) scope: Option<Opener>,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Summary {
    pub(crate) exploration: String,
    #[serde(flatten)]
    pub(crate) extent: Extent,
    pub(crate) engine: Engine,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) device: Option<Device>,
    pub(crate) order: Order,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) shape: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[schemars(
        description = "Why the exploration stopped short of closing: each budget that ran out and each limit that blocked events, with how many; on a direct path, why the path stopped. Empty once the exploration closes."
    )]
    pub(crate) stop: Vec<Stop>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "On a direct path, whether it reached its goal.")]
    pub(crate) reached: Option<bool>,
    #[schemars(
        description = "Work steps the engine took; metal counts a step for each match its grounding read."
    )]
    pub(crate) work: usize,
    pub(crate) configuration: usize,
    pub(crate) event: usize,
    pub(crate) inferred: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "The longest shortest path from the start; metal keeps no paths.")]
    pub(crate) depth: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Whether a run can go on forever: true once a cycle is found, false once the exploration closes without one."
    )]
    pub(crate) endless: Option<bool>,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Answer {
    #[serde(flatten)]
    pub(crate) summary: Summary,
    #[schemars(
        description = "Configurations with no supported event out of them. In an open exploration some are unexplored rather than ends."
    )]
    pub(crate) end: Vec<End>,
    pub(crate) more: usize,
    pub(crate) rule: Vec<Activity>,
}

pub(crate) fn brief(explored: &Explored) -> Summary {
    match explored {
        Explored::Exploration(exploration) => Summary {
            exploration: exploration.name(),
            extent: exploration.extent(),
            engine: exploration.engine,
            device: None,
            order: exploration.order,
            shape: exploration.shape.map(|shape| format!("{shape:016x}")),
            stop: exploration.stop.clone(),
            reached: (exploration.mode == Mode::Path).then_some(exploration.reached),
            work: exploration.work,
            configuration: exploration.configuration.len(),
            event: exploration.event.len(),
            inferred: (0..exploration.event.len())
                .filter(|&index| exploration.inferred(index))
                .count(),
            depth: exploration.depth.iter().flatten().copied().max(),
            endless: exploration.endless(),
        },
        Explored::Survey(survey) => Summary {
            exploration: format!("x{}", survey.key),
            extent: Extent {
                mode: Mode::Plain,
                closed: survey.closed,
            },
            engine: Engine::Metal,
            device: Some(survey.device),
            order: survey.order,
            shape: survey.shape.map(|shape| format!("{shape:016x}")),
            stop: survey.stop.clone(),
            reached: None,
            work: survey.work,
            configuration: survey.configuration,
            event: usize::try_from(survey.event).unwrap_or(usize::MAX),
            inferred: 0,
            depth: None,
            endless: survey.endless,
        },
    }
}

// Every rule's supported events are gathered in one pass, since asking each rule for its own would
// read every event once for every rule.
fn activity(exploration: &Exploration) -> Vec<Activity> {
    let mut firing = vec![Vec::new(); exploration.rule.len()];
    for (index, event) in exploration.event.iter().enumerate() {
        if event.supported {
            firing[event.rule].push(index);
        }
    }
    firing
        .into_iter()
        .enumerate()
        .map(|(index, event)| Activity {
            handle: Handle::Rule(index).to_string(),
            text: exploration.rule[index].text.clone(),
            fired: event.len(),
            inferred: event
                .iter()
                .filter(|&&event| exploration.inferred(event))
                .count(),
            first: event.first().map(|&event| Handle::Event(event).to_string()),
            scope: exploration.rule[index].scope,
        })
        .collect()
}

fn summary(explored: &Explored, limit: usize) -> Answer {
    let (count, end, rule) = match explored {
        Explored::Exploration(exploration) => {
            let leaf = exploration.leaf().collect::<Vec<_>>();
            let end = leaf
                .iter()
                .take(limit)
                .map(|&index| End {
                    handle: Some(Handle::Configuration(index).to_string()),
                    scope: render::scope(&exploration.configuration[index]),
                    text: render::configuration(exploration, index),
                })
                .collect();
            (leaf.len(), end, activity(exploration))
        }
        Explored::Survey(survey) => {
            let end = survey
                .end
                .iter()
                .take(limit)
                .map(|configuration| End {
                    handle: None,
                    scope: render::scope(configuration),
                    text: render::text(&survey.rule, configuration),
                })
                .collect();
            (survey.end.len(), end, Vec::new())
        }
    };
    Answer {
        summary: brief(explored),
        more: count.saturating_sub(limit),
        end,
        rule,
    }
}

pub(crate) fn answer(request: &Request, context: &mut Context<'_>) -> Result<Answer, Failure> {
    let explored = context.explored(&request.recording)?;
    Ok(summary(&explored, request.limit))
}

// The words a list uses: one, two joined by and, or more with commas before the last and.
fn list(item: &[String]) -> String {
    match item {
        [] => String::new(),
        [only] => only.clone(),
        [first @ .., last] => format!("{} and {last}", first.join(", ")),
    }
}

fn plural(count: usize, noun: &str) -> String {
    format!("{count} {noun}{}", if count == 1 { "" } else { "s" })
}

// Why an exploration stopped, as its answers say it: each reason, then the budgets and limits to
// raise so it goes on. Metal's configuration limit is its device's capacity unless a question kept
// fewer, and then says so.
pub(crate) fn reason(stop: &[Stop], device: Option<Device>) -> String {
    let said = stop
        .iter()
        .map(|&stop| match (stop, device) {
            (
                Stop::Limit {
                    bound: Bound::Configuration,
                    value,
                    blocked,
                },
                Some(device),
            ) if value == device.capacity() => format!(
                "the capacity of {} ({}) blocked {}",
                device.name(),
                plural(value, "configuration"),
                plural(blocked, "event")
            ),
            (stop, _) => stop.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", and ");
    let mut raise = Vec::new();
    for flag in stop.iter().filter_map(|stop| stop.raise()) {
        let flag = format!("--{flag}");
        if !raise.contains(&flag) {
            raise.push(flag);
        }
    }
    if raise.is_empty() {
        return said;
    }
    format!("{said}; raise {}", list(&raise))
}

pub(crate) fn state(summary: &Summary) -> String {
    let status = match (summary.extent.mode, summary.extent.closed, summary.reached) {
        (Mode::Path, _, Some(true)) => "path reached its goal".to_owned(),
        (Mode::Path, _, _) => format!("path stopped: {}", reason(&summary.stop, None)),
        (Mode::Exhaustive | Mode::Plain, true, _) => "closed".to_owned(),
        (Mode::Exhaustive | Mode::Plain, false, _) => {
            format!("open: {}", reason(&summary.stop, summary.device))
        }
    };
    let mut part = vec![summary.exploration.clone(), status];
    match (summary.extent.mode, summary.engine, summary.device) {
        (Mode::Plain, Engine::Metal, Some(device)) => {
            part.extend(["plain".to_owned(), format!("metal on {}", device.name())]);
        }
        (Mode::Plain, _, _) => part.push("plain".to_owned()),
        (Mode::Exhaustive, Engine::Interpreter, _) => part.push("interpreter".to_owned()),
        _ => {}
    }
    part.extend([
        render::count(summary.configuration, "configuration"),
        format!(
            "{}, {} inferred",
            render::count(summary.event, "event"),
            summary.inferred
        ),
    ]);
    part.extend(summary.depth.map(|depth| format!("depth {depth}")));
    part.push(format!("work {}", summary.work));
    part.extend(summary.shape.as_ref().map(|shape| format!("shape {shape}")));
    if summary.endless == Some(true) {
        part.push("a run can go on forever".to_owned());
    }
    part.join(" · ")
}

impl Answer {
    pub(crate) fn text(&self) -> String {
        let mut line = vec![state(&self.summary)];
        let extent = self.summary.extent;
        let heading = match extent.kind() {
            Kind::Closed => "end",
            Kind::Open => "leaf",
            Kind::Path => "stop",
        };
        if self.end.is_empty() {
            line.push(match self.more {
                0 => format!("{heading:<6} none"),
                more => format!(
                    "{heading:<6} {} not listed",
                    render::count(more, "configuration")
                ),
            });
        }
        for (position, end) in self.end.iter().enumerate() {
            let label = if position == 0 { heading } else { "" };
            line.push(match &end.handle {
                Some(handle) => format!("{label:<6} {handle:<5} {}", end.text),
                None => format!("{label:<6} {}", end.text),
            });
        }
        if self.more > 0 && !self.end.is_empty() {
            line.push(format!("       and {} more", self.more));
        }
        let width = self
            .rule
            .iter()
            .map(|rule| rule.text.chars().count())
            .max()
            .unwrap_or(0)
            .min(48);
        for (position, rule) in self.rule.iter().enumerate() {
            let label = if position == 0 { "rule" } else { "" };
            let fired = match (rule.fired, rule.inferred) {
                (0, _) => extent.never().to_owned(),
                (fired, 0) => fired.to_string(),
                (fired, inferred) => format!("{fired}, {inferred} inferred"),
            };
            let scope = match rule.scope {
                Some(Opener::Rule(opener)) => {
                    format!("   in the scope {} opens", Handle::Rule(opener))
                }
                Some(Opener::Program) => "   in a scope the program opens".to_owned(),
                None => String::new(),
            };
            line.push(format!(
                "{label:<6} {:<5} {:<width$}   {fired}{scope}",
                rule.handle, rule.text
            ));
        }
        line.join("\n")
    }
}
