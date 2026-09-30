use crate::claim::{self, Claim, Verdict};
use crate::context::Context;
use crate::explore::{self, Summary};
use crate::explored::Explored;
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
    Diagnostic {
        code: failure
            .diagnostic
            .unwrap_or_else(|| render::name(failure.code)),
        message: failure.message,
        location: failure.location,
    }
}

// A program that does not assemble is answered with its diagnostic, and only assembling fails with
// a source or library code; every other failure is the request's.
pub(crate) fn answer(request: &Request, context: &mut Context<'_>) -> Result<Answer, Failure> {
    for claim in &request.claim {
        claim::admit(claim, &request.recording)?;
    }
    let explored = match context.explored(&request.recording) {
        Ok(explored) => explored,
        Err(failure) if matches!(failure.code, Code::Source | Code::Library) => {
            return Ok(Answer {
                diagnostic: vec![diagnostic(failure)],
                summary: None,
                claim: Vec::new(),
            });
        }
        Err(failure) => return Err(failure),
    };
    Ok(Answer {
        diagnostic: Vec::new(),
        summary: Some(explore::brief(&explored)),
        claim: request
            .claim
            .iter()
            .map(|claim| match &explored {
                Explored::Exploration(exploration) => claim::evaluate(claim, exploration),
                Explored::Survey(survey) => claim::survey(claim, survey),
            })
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
            line.push(verdict.text());
        }
        if let Some(summary) = &self.summary {
            line.push(explore::state(summary));
        }
        line.join("\n")
    }
}
