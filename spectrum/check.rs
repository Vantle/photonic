use crate::claim::{self, Claim, Verdict};
use crate::context::Context;
use crate::explore::{self, Summary};
use crate::failure::{Code, Failure, Location};
use crate::recording::Recording;
use crate::render;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(
    description = "Check a program: report its diagnostics as data, then explore it and answer each claim with holds, fails or unknown and the evidence."
)]
pub struct Request {
    #[serde(flatten)]
    pub recording: Recording,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claim: Vec<Claim>,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Diagnostic {
    pub(crate) code: String,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) location: Option<Location>,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Answer {
    pub(crate) diagnostic: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) summary: Option<Summary>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) claim: Vec<Verdict>,
}

fn diagnostic(failure: Failure) -> Diagnostic {
    let code = failure
        .diagnostic
        .clone()
        .unwrap_or_else(|| render::name(failure.code));
    Diagnostic {
        code,
        message: failure.message,
        location: failure.location,
    }
}

pub(crate) fn answer(request: &Request, context: &mut Context<'_>) -> Result<Answer, Failure> {
    let exploration = match (&request.recording.program, &request.recording.exploration) {
        (Some(program), None) => match program.assemble(context.reader) {
            Ok(source) => context.explore(&source, &request.recording)?,
            Err(failure) if matches!(failure.code, Code::Request | Code::File) => {
                return Err(failure);
            }
            Err(failure) => {
                return Ok(Answer {
                    diagnostic: vec![diagnostic(failure)],
                    summary: None,
                    claim: Vec::new(),
                });
            }
        },
        _ => context.exploration(&request.recording)?,
    };
    Ok(Answer {
        diagnostic: Vec::new(),
        summary: Some(explore::brief(&exploration)),
        claim: request
            .claim
            .iter()
            .map(|claim| claim::evaluate(claim, &exploration))
            .collect::<Result<_, _>>()?,
    })
}

impl Answer {
    pub(crate) fn passed(&self) -> bool {
        self.diagnostic.is_empty()
            && self
                .claim
                .iter()
                .all(|verdict| verdict.answer == claim::Answer::Holds)
    }

    pub(crate) fn text(&self) -> String {
        let mut line = Vec::new();
        for diagnostic in &self.diagnostic {
            let place = diagnostic
                .location
                .as_ref()
                .map(|location| {
                    format!("{}:{}:{}   ", location.file, location.line, location.column)
                })
                .unwrap_or_default();
            line.push(format!(
                "{place}error {}   {}",
                diagnostic.code, diagnostic.message
            ));
        }
        if self.diagnostic.is_empty() {
            line.push("no diagnostics".to_owned());
        }
        for verdict in &self.claim {
            line.push(explore::verdict(verdict));
        }
        if let Some(summary) = &self.summary {
            line.push(explore::state(summary));
        }
        line.join("\n")
    }
}
