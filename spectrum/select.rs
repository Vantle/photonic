use crate::context::Context;
use crate::failure::Failure;
use crate::handle::Handle;
use crate::pattern::{self, Pattern};
use crate::recording::Recording;
use crate::render;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const LIMIT: usize = 20;

fn limit() -> usize {
    LIMIT
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(
    description = "Find where a Photonic pattern happens: coherence patterns select configurations, rule patterns select events."
)]
pub struct Request {
    #[serde(flatten)]
    pub recording: Recording,
    #[schemars(
        description = "B selects a coherence holding B; B.X one holding both; B, C two different coherences; ().([A] B) a coherence holding that rule value; (K, [K] L) a scope holding K and that rule; [B, C] D the events applying that rule."
    )]
    pub pattern: String,
    #[serde(default = "limit")]
    pub limit: usize,
    #[serde(default)]
    #[schemars(description = "Matches to skip, to page through a long answer.")]
    pub offset: usize,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Kind {
    Configuration,
    Event,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Found {
    pub(crate) handle: String,
    pub(crate) text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) occurrence: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[schemars(description = "The scope frames the pattern's scopes matched.")]
    pub(crate) frame: Vec<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub(crate) unsupported: bool,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Answer {
    pub(crate) exploration: String,
    pub(crate) pattern: String,
    pub(crate) kind: Kind,
    pub(crate) total: usize,
    pub(crate) found: Vec<Found>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) next: Option<usize>,
}

pub(crate) fn answer(request: &Request, context: &mut Context<'_>) -> Result<Answer, Failure> {
    let pattern = Pattern::read(&request.pattern)?;
    let exploration = context.exploration(&request.recording)?;
    let (kind, all) = match &pattern {
        Pattern::Configuration(body) => (
            Kind::Configuration,
            (0..exploration.configuration.len())
                .filter_map(|index| {
                    let found = pattern::assign(
                        body,
                        &exploration.configuration[index],
                        exploration.rule.as_slice(),
                    )?;
                    Some(Found {
                        handle: Handle::Configuration(index).to_string(),
                        text: render::configuration(&exploration, index),
                        occurrence: found
                            .coherence
                            .iter()
                            .flat_map(|entry| &entry.occurrence)
                            .map(|&id| Handle::Occurrence(index, id).to_string())
                            .collect(),
                        frame: found
                            .frame
                            .iter()
                            .map(|&frame| Handle::Frame(index, frame).to_string())
                            .collect(),
                        unsupported: !exploration.configuration[index].supported,
                    })
                })
                .collect::<Vec<_>>(),
        ),
        Pattern::Rule(definition) => {
            let rule = pattern::rule(
                definition,
                exploration.rule.as_slice(),
                exploration.rule.len(),
            );
            (
                Kind::Event,
                (0..exploration.event.len())
                    .filter(|&index| rule.contains(&exploration.event[index].rule))
                    .map(|index| {
                        let entry = &exploration.event[index];
                        Found {
                            handle: Handle::Event(index).to_string(),
                            text: format!(
                                "{} {} · {} → {} {}",
                                Handle::Rule(entry.rule),
                                exploration.rule[entry.rule].text,
                                Handle::Configuration(entry.source),
                                Handle::Configuration(entry.target),
                                render::configuration(&exploration, entry.target)
                            ),
                            occurrence: Vec::new(),
                            frame: Vec::new(),
                            unsupported: !entry.supported,
                        }
                    })
                    .collect::<Vec<_>>(),
            )
        }
    };
    let total = all.len();
    let end = request.offset.saturating_add(request.limit).min(total);
    Ok(Answer {
        exploration: exploration.name(),
        pattern: request.pattern.clone(),
        kind,
        total,
        found: all
            .into_iter()
            .skip(request.offset)
            .take(request.limit)
            .collect(),
        next: (end < total).then_some(end),
    })
}

impl Answer {
    pub(crate) fn text(&self) -> String {
        let noun = match self.kind {
            Kind::Configuration => "configuration",
            Kind::Event => "event",
        };
        let mut line = vec![format!(
            "{} · {} {}",
            self.exploration,
            render::count(self.total, noun),
            if self.total == 0 { "match" } else { "matching" }
        )];
        line[0].push_str(&format!(" {}", self.pattern));
        let width = self
            .found
            .iter()
            .map(|found| found.text.chars().count())
            .max()
            .unwrap_or(0);
        for found in &self.found {
            let note = if found.unsupported {
                "   unsupported"
            } else {
                ""
            };
            let matched = found
                .frame
                .iter()
                .chain(&found.occurrence)
                .map(String::as_str)
                .collect::<Vec<_>>();
            if matched.is_empty() {
                line.push(format!("{:<6} {}{note}", found.handle, found.text));
                continue;
            }
            line.push(format!(
                "{:<6} {:<width$}   matched {}{note}",
                found.handle,
                found.text,
                matched.join(" ")
            ));
        }
        if let Some(next) = self.next {
            line.push(format!("more: offset {next}"));
        }
        line.join("\n")
    }
}
