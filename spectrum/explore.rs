use crate::claim::{self, Claim, Verdict};
use crate::context::Context;
use crate::exploration::Exploration;
use crate::failure::Failure;
use crate::handle::Handle;
use crate::recording::{Mode, Order, Recording};
use crate::render;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn limit() -> usize {
    12
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(
    description = "Explore every future of a program and summarize it: counts, end configurations, rule activity and claims."
)]
pub struct Request {
    #[serde(flatten)]
    pub recording: Recording,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claim: Vec<Claim>,
    #[serde(default = "limit")]
    #[schemars(description = "End configurations listed; the rest are counted.")]
    pub limit: usize,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct End {
    pub handle: String,
    pub scope: bool,
    pub text: String,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Activity {
    pub handle: String,
    pub text: String,
    pub fired: usize,
    pub inferred: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Summary {
    pub exploration: String,
    pub mode: Mode,
    pub order: Order,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape: Option<String>,
    pub complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reached: Option<bool>,
    pub work: usize,
    pub configuration: usize,
    pub event: usize,
    pub inferred: usize,
    pub depth: usize,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Answer {
    #[serde(flatten)]
    pub summary: Summary,
    #[schemars(
        description = "Configurations with no supported event out of them. In an open exploration some are unexplored rather than ends."
    )]
    pub end: Vec<End>,
    pub more: usize,
    pub rule: Vec<Activity>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub claim: Vec<Verdict>,
}

pub(crate) fn brief(exploration: &Exploration) -> Summary {
    Summary {
        exploration: exploration.name(),
        mode: exploration.mode,
        order: exploration.order,
        shape: exploration.shape.map(|shape| format!("{shape:016x}")),
        complete: exploration.closed,
        reached: (exploration.mode == Mode::Path).then_some(exploration.reached),
        work: exploration.work,
        configuration: exploration.configuration.len(),
        event: exploration.event.len(),
        inferred: (0..exploration.event.len())
            .filter(|&index| exploration.inferred(index))
            .count(),
        depth: exploration
            .depth
            .iter()
            .flatten()
            .copied()
            .max()
            .unwrap_or(0),
    }
}

fn summary(exploration: &Exploration, limit: usize) -> Answer {
    let end = exploration.leaf().collect::<Vec<_>>();
    Answer {
        summary: brief(exploration),
        more: end.len().saturating_sub(limit),
        end: end
            .iter()
            .take(limit)
            .map(|&index| End {
                handle: Handle::Configuration(index).to_string(),
                scope: render::scope(exploration, index),
                text: render::configuration(exploration, index),
            })
            .collect(),
        rule: (0..exploration.rule.len())
            .map(|index| {
                let event = exploration.firing(index).collect::<Vec<_>>();
                Activity {
                    handle: Handle::Rule(index).to_string(),
                    text: render::brief(exploration, index),
                    fired: event.len(),
                    inferred: event
                        .iter()
                        .filter(|&&event| exploration.inferred(event))
                        .count(),
                    first: event.first().map(|&event| Handle::Event(event).to_string()),
                    scope: exploration.rule[index]
                        .scope
                        .map(|scope| Handle::Rule(scope).to_string()),
                }
            })
            .collect(),
        claim: Vec::new(),
    }
}

pub fn answer(request: &Request, context: &mut Context<'_>) -> Result<Answer, Failure> {
    let exploration = context.exploration(&request.recording)?;
    let mut answer = summary(&exploration, request.limit);
    answer.claim = request
        .claim
        .iter()
        .map(|claim| claim::evaluate(claim, &exploration))
        .collect::<Result<_, _>>()?;
    Ok(answer)
}

pub(crate) fn state(summary: &Summary) -> String {
    let status = match (summary.mode, summary.complete, summary.reached) {
        (Mode::Path, _, Some(true)) => "path reached its goal",
        (Mode::Path, _, _) => "path stopped",
        (Mode::Exhaustive, true, _) => "closed",
        (Mode::Exhaustive, false, _) => "open: a budget stopped it",
    };
    let mut part = vec![
        summary.exploration.clone(),
        status.to_owned(),
        render::count(summary.configuration, "configuration"),
        format!(
            "{}, {} inferred",
            render::count(summary.event, "event"),
            summary.inferred
        ),
        format!("depth {}", summary.depth),
        format!("work {}", summary.work),
    ];
    if let Some(shape) = &summary.shape {
        part.push(format!("shape {shape}"));
    }
    part.join(" · ")
}

pub(crate) fn verdict(verdict: &Verdict) -> String {
    let kind = render::name(verdict.claim.kind);
    let answer = render::name(verdict.answer);
    let exact = if verdict.claim.exact { " exactly" } else { "" };
    let mut line = format!("{kind} {}{exact}   {answer}", verdict.claim.pattern);
    if let Some(witness) = &verdict.witness {
        line.push_str(&format!("   {witness}"));
        if !verdict.path.is_empty() {
            line.push_str(&format!(" by {}", verdict.path.join(" ")));
        }
    }
    line.push_str(&format!("   {}", verdict.reason));
    line
}

impl Answer {
    pub fn text(&self) -> String {
        let mut line = vec![state(&self.summary)];
        if self.end.is_empty() {
            line.push("end    none".to_owned());
        }
        let heading = match (self.summary.mode, self.summary.complete) {
            (Mode::Exhaustive, true) => "end",
            (Mode::Exhaustive, false) => "leaf",
            (Mode::Path, _) => "stop",
        };
        for (position, end) in self.end.iter().enumerate() {
            let label = if position == 0 { heading } else { "" };
            line.push(format!("{label:<6} {:<5} {}", end.handle, end.text));
        }
        if self.more > 0 {
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
                (0, _) => "never".to_owned(),
                (fired, 0) => fired.to_string(),
                (fired, inferred) => format!("{fired}, {inferred} inferred"),
            };
            let scope = rule
                .scope
                .as_ref()
                .map(|scope| format!("   in the scope {scope} opens"))
                .unwrap_or_default();
            line.push(format!(
                "{label:<6} {:<5} {:<width$}   {fired}{scope}",
                rule.handle, rule.text
            ));
        }
        for (position, claim) in self.claim.iter().enumerate() {
            let label = if position == 0 { "claim" } else { "" };
            line.push(format!("{label:<6} {}", verdict(claim)));
        }
        line.join("\n")
    }
}
