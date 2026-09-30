use crate::exploration::{self, Exploration};
use crate::explore;
use crate::failure::{Code, Failure};
use crate::handle::Handle;
use crate::pattern::{self, Pattern};
use crate::recording::Mode;
use crate::render;
use crate::survey::Survey;
use frontend::source::Program;
use photonic::prism::Outcome;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Reach,
    Avoid,
    Always,
    Inevitable,
    Outcome,
    End,
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(
    description = "A claim about every future. reach: some configuration matches; avoid: none does; always: every one does; inevitable: every run reaches a match; outcome: every end configuration matches; end: every run ends, and ends at a match."
)]
pub struct Claim {
    pub kind: Kind,
    #[schemars(
        description = "A Photonic pattern, matched by containment as a rule's input is; with exact, a complete configuration as Prism reads it."
    )]
    pub pattern: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[schemars(description = "Compare whole configurations, as Prism does.")]
    pub exact: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[schemars(description = "With exact, the target also lists every loaded root rule.")]
    pub preserve: bool,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Answer {
    Holds,
    Fails,
    Unknown,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Verdict {
    pub(crate) claim: Claim,
    pub(crate) answer: Answer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) witness: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) path: Vec<String>,
    pub(crate) reason: String,
}

struct Evidence {
    answer: Answer,
    witness: Option<usize>,
    path: Vec<usize>,
    reason: String,
}

impl Evidence {
    fn at(answer: Answer, exploration: &Exploration, witness: usize, reason: String) -> Self {
        Self {
            answer,
            path: exploration.path(witness).unwrap_or_default(),
            witness: Some(witness),
            reason,
        }
    }

    fn plain(answer: Answer, reason: impl Into<String>) -> Self {
        Self {
            answer,
            witness: None,
            path: Vec::new(),
            reason: reason.into(),
        }
    }
}

fn shallowest(exploration: &Exploration, candidate: impl Iterator<Item = usize>) -> Option<usize> {
    candidate.min_by_key(|&index| (exploration.depth[index].unwrap_or(usize::MAX), index))
}

fn supported(exploration: &Exploration) -> impl Iterator<Item = usize> + '_ {
    (0..exploration.configuration.len()).filter(|&index| exploration.configuration[index].supported)
}

const PATH: &str = "a direct path follows one run of many, so it cannot settle this";

fn open(exploration: &Exploration) -> String {
    format!(
        "the exploration stopped after {}: {}",
        render::count(exploration.configuration.len(), "configuration"),
        explore::reason(&exploration.stop, None)
    )
}

fn unexplored(exploration: &Exploration) -> String {
    if exploration.mode == Mode::Path {
        return PATH.to_owned();
    }
    open(exploration)
}

// How reasons speak of what a claim looks for: a match of its pattern, or its exact target.
struct Word {
    noun: &'static str,
    is: &'static str,
    not: &'static str,
}

fn word(claim: &Claim) -> Word {
    if claim.exact {
        return Word {
            noun: "the target",
            is: "is the target",
            not: "is not the target",
        };
    }
    Word {
        noun: "a match",
        is: "matches",
        not: "does not match",
    }
}

fn inevitable(exploration: &Exploration, matched: &[bool], word: &Word) -> Evidence {
    if matched.first().copied().unwrap_or(false) {
        return Evidence::plain(Answer::Holds, format!("the start {}", word.is));
    }
    let avoided = matched
        .iter()
        .zip(&exploration.configuration)
        .map(|(&matched, configuration)| !matched && configuration.supported)
        .collect::<Vec<_>>();
    if let Some((entry, route)) = exploration.cycle(&avoided) {
        return Evidence {
            answer: Answer::Fails,
            witness: Some(entry),
            path: route,
            reason: format!(
                "a run can cycle through s{entry} forever without {}",
                word.noun
            ),
        };
    }
    if !exploration.closed {
        return Evidence::plain(Answer::Unknown, open(exploration));
    }
    let (parent, depth) = exploration::tree(&exploration.outgoing, &exploration.event, |node| {
        avoided[node]
    });
    let end = exploration
        .leaf()
        .filter(|&index| depth[index].is_some())
        .min_by_key(|&index| (depth[index], index));
    let Some(index) = end else {
        return Evidence::plain(Answer::Holds, format!("every run reaches {}", word.noun));
    };
    Evidence {
        answer: Answer::Fails,
        witness: Some(index),
        path: exploration::trail(&parent, &exploration.event, index),
        reason: format!("a run ends at s{index} without {}", word.noun),
    }
}

// Every run ends, and at a match: a cycle any run reaches fails it at once, and a run that ends
// without a match fails it once the exploration closes.
fn end(exploration: &Exploration, matched: &[bool], word: &Word) -> Evidence {
    let every = vec![true; exploration.configuration.len()];
    if let Some((entry, route)) = exploration.cycle(&every) {
        return Evidence {
            answer: Answer::Fails,
            witness: Some(entry),
            path: route,
            reason: format!("a run can cycle through s{entry} forever"),
        };
    }
    if !exploration.closed {
        return Evidence::plain(Answer::Unknown, open(exploration));
    }
    let stray = exploration.leaf().filter(|&index| !matched[index]);
    match shallowest(exploration, stray) {
        Some(index) => Evidence::at(
            Answer::Fails,
            exploration,
            index,
            format!("a run ends at s{index} without {}", word.noun),
        ),
        None => Evidence::plain(Answer::Holds, format!("every run ends at {}", word.noun)),
    }
}

fn body(claim: &Claim) -> Result<pattern::Body, Failure> {
    match Pattern::read(&claim.pattern)? {
        Pattern::Configuration(body) => Ok(body),
        Pattern::Rule(_) => Err(Failure::new(
            Code::Claim,
            "a claim is about configurations; write a pattern of coherences and scopes, such as False.Extra",
        )),
    }
}

fn target(claim: &Claim) -> Result<Program, Failure> {
    crate::subject::lower("pattern", &claim.pattern, Code::Target)
}

// Which configurations a claim's pattern matches, or, when exact, which one is its target.
fn matched(claim: &Claim, exploration: &Exploration) -> Result<Vec<bool>, Failure> {
    if !claim.exact {
        let body = body(claim)?;
        return Ok(exploration
            .configuration
            .iter()
            .map(|configuration| {
                configuration.supported
                    && pattern::assign(&body, configuration, exploration.rule.as_slice()).is_some()
            })
            .collect());
    }
    if exploration.mode == Mode::Path {
        return Err(Failure::new(
            Code::Claim,
            "a direct path checks an exact target as its goal; explore in path mode with goal",
        ));
    }
    let verdict = exploration
        .verdict(&target(claim)?, claim.preserve)
        .ok_or_else(|| Failure::new(Code::Claim, "this exploration cannot check exact targets"))?;
    let mut matched = vec![false; exploration.configuration.len()];
    if let (Outcome::Reached, Some(witness)) = (verdict.outcome, verdict.witness) {
        matched[witness] = true;
    }
    Ok(matched)
}

fn decide(claim: &Claim, exploration: &Exploration, matched: &[bool]) -> Evidence {
    let word = word(claim);
    let found = shallowest(
        exploration,
        supported(exploration).filter(|&index| matched[index]),
    );
    let missing = shallowest(
        exploration,
        supported(exploration).filter(|&index| !matched[index]),
    );
    let path = exploration.mode == Mode::Path;
    let closed = exploration.settled();
    match claim.kind {
        Kind::Reach | Kind::Avoid => {
            let (present, absent) = if claim.kind == Kind::Reach {
                (Answer::Holds, Answer::Fails)
            } else {
                (Answer::Fails, Answer::Holds)
            };
            match found {
                Some(index) => {
                    Evidence::at(present, exploration, index, format!("s{index} {}", word.is))
                }
                None if closed => Evidence::plain(
                    absent,
                    format!("no configuration {}, and the exploration closed", word.is),
                ),
                None => Evidence::plain(Answer::Unknown, unexplored(exploration)),
            }
        }
        Kind::Always => match missing {
            Some(index) => Evidence::at(
                Answer::Fails,
                exploration,
                index,
                format!("s{index} {}", word.not),
            ),
            None if closed => {
                Evidence::plain(Answer::Holds, format!("every configuration {}", word.is))
            }
            None => Evidence::plain(Answer::Unknown, unexplored(exploration)),
        },
        Kind::Inevitable | Kind::Outcome | Kind::End if path => {
            Evidence::plain(Answer::Unknown, PATH)
        }
        Kind::Inevitable => inevitable(exploration, matched, &word),
        Kind::Outcome if !exploration.closed => Evidence::plain(
            Answer::Unknown,
            format!(
                "end configurations are known once the exploration closes, and {}",
                explore::reason(&exploration.stop, None)
            ),
        ),
        Kind::Outcome => {
            let stray = exploration.leaf().filter(|&index| !matched[index]);
            match shallowest(exploration, stray) {
                Some(index) => Evidence::at(
                    Answer::Fails,
                    exploration,
                    index,
                    format!("s{index} ends without {}", word.noun),
                ),
                None => Evidence::plain(
                    Answer::Holds,
                    format!("every end configuration {}", word.is),
                ),
            }
        }
        Kind::End => end(exploration, matched, &word),
    }
}

fn verdict(claim: &Claim, evidence: Evidence) -> Verdict {
    Verdict {
        claim: claim.clone(),
        answer: evidence.answer,
        witness: evidence
            .witness
            .map(|index| Handle::Configuration(index).to_string()),
        path: evidence
            .path
            .iter()
            .map(|&event| Handle::Event(event).to_string())
            .collect(),
        reason: evidence.reason,
    }
}

pub(crate) fn evaluate(claim: &Claim, exploration: &Exploration) -> Result<Verdict, Failure> {
    let matched = matched(claim, exploration)?;
    Ok(verdict(claim, decide(claim, exploration, &matched)))
}

// A survey keeps only its ends and whether a run can go on forever, so it answers outcome and end,
// and names an end by its text, since it numbers nothing.
pub(crate) fn survey(claim: &Claim, survey: &Survey) -> Result<Verdict, Failure> {
    if !matches!(claim.kind, Kind::Outcome | Kind::End) {
        return Err(Failure::new(
            Code::Claim,
            "metal keeps only counts, ends and cycles, so it answers outcome and end; ask reach, avoid, always and inevitable with laser",
        ));
    }
    let matched = if claim.exact {
        survey.target(&target(claim)?, claim.preserve)
    } else {
        let body = body(claim)?;
        survey
            .end
            .iter()
            .map(|configuration| {
                pattern::assign(&body, configuration, survey.rule.as_slice()).is_some()
            })
            .collect()
    };
    let word = word(claim);
    let stray = matched.iter().position(|&matched| !matched);
    let evidence = match (claim.kind, stray) {
        (Kind::End, _) if survey.endless == Some(true) => {
            Evidence::plain(Answer::Fails, "a run can go on forever")
        }
        _ if !survey.closed => Evidence::plain(
            Answer::Unknown,
            format!(
                "end configurations are known once the exploration closes, and {}",
                explore::reason(&survey.stop, Some(survey.device))
            ),
        ),
        (_, Some(index)) => Evidence::plain(
            Answer::Fails,
            format!(
                "a run ends at {} without {}",
                render::text(&survey.rule, &survey.end[index]),
                word.noun
            ),
        ),
        (Kind::End, None) => {
            Evidence::plain(Answer::Holds, format!("every run ends at {}", word.noun))
        }
        (_, None) => Evidence::plain(
            Answer::Holds,
            format!("every end configuration {}", word.is),
        ),
    };
    Ok(verdict(claim, evidence))
}

impl Verdict {
    pub(crate) fn text(&self) -> String {
        let kind = render::name(self.claim.kind);
        let answer = render::name(self.answer);
        let exact = if self.claim.exact { " exactly" } else { "" };
        let mut line = format!("{kind} {}{exact}   {answer}", self.claim.pattern);
        if let Some(witness) = &self.witness {
            line.push_str(&format!("   {witness}"));
            if !self.path.is_empty() {
                line.push_str(&format!(" by {}", self.path.join(" ")));
            }
        }
        line.push_str(&format!("   {}", self.reason));
        line
    }
}
