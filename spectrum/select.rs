use crate::context::Context;
use crate::failure::Failure;
use crate::handle::Handle;
use crate::pattern::{self, Pattern};
use crate::recording::Recording;
use crate::render;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn limit() -> usize {
    20
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(
    description = "Find where a Photonic pattern happens: coherence patterns select configurations, rule patterns select events."
)]
pub struct Request {
    #[serde(flatten)]
    pub recording: Recording,
    #[schemars(
        description = "B selects a coherence holding B; B.X one holding both; B, C two different coherences; ([A] B) a coherence holding that rule value; [B, C] D the events applying that rule."
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
pub enum Kind {
    Configuration,
    Event,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Found {
    pub handle: String,
    pub text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub occurrence: Vec<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub unsupported: bool,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Answer {
    pub exploration: String,
    pub pattern: String,
    pub kind: Kind,
    pub total: usize,
    pub found: Vec<Found>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<usize>,
}

pub fn answer(request: &Request, context: &mut Context<'_>) -> Result<Answer, Failure> {
    let pattern = Pattern::read(&request.pattern)?;
    let exploration = context.exploration(&request.recording)?;
    let (kind, all) = match &pattern {
        Pattern::Coherence(item) => (
            Kind::Configuration,
            (0..exploration.configuration.len())
                .filter_map(|index| {
                    let found = pattern::assign(item, &exploration, index)?;
                    Some(Found {
                        handle: Handle::Configuration(index).to_string(),
                        text: render::configuration(&exploration, index),
                        occurrence: found
                            .iter()
                            .flat_map(|entry| &entry.occurrence)
                            .map(|&id| Handle::Occurrence(index, id).to_string())
                            .collect(),
                        unsupported: !exploration.configuration[index].supported,
                    })
                })
                .collect::<Vec<_>>(),
        ),
        Pattern::Rule(definition) => {
            let rule = pattern::rule(definition, &exploration);
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
    pub fn text(&self) -> String {
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
            if found.occurrence.is_empty() {
                line.push(format!("{:<6} {}{note}", found.handle, found.text));
                continue;
            }
            line.push(format!(
                "{:<6} {:<width$}   matched {}{note}",
                found.handle,
                found.text,
                found.occurrence.join(" ")
            ));
        }
        if let Some(next) = self.next {
            line.push(format!("more: offset {next}"));
        }
        line.join("\n")
    }
}
