use crate::exploration::Exploration;
use crate::failure::{Code, Failure};
use crate::handle::Handle;
use crate::pattern::{self, Pattern};
use crate::recording::Mode;
use crate::render;
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
}

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[schemars(
    description = "A claim about every future. reach: some configuration matches; avoid: none does; always: every one does; inevitable: every run reaches a match; outcome: every end configuration matches."
)]
pub struct Claim {
    pub kind: Kind,
    #[schemars(
        description = "A Photonic pattern, matched by containment as a rule's input is; with exact, a complete configuration as Prism reads it."
    )]
    pub pattern: String,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[schemars(description = "Compare whole configurations, as Prism does; reach and avoid only.")]
    pub exact: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[schemars(description = "With exact, the target also lists every loaded root rule.")]
    pub preserve: bool,
}

#[derive(Clone, Copy, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Answer {
    Holds,
    Fails,
    Unknown,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Verdict {
    pub claim: Claim,
    pub answer: Answer,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub witness: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<String>,
    pub reason: String,
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
        "the exploration stopped at its budget after {}",
        render::count(exploration.configuration.len(), "configuration")
    )
}

fn unexplored(exploration: &Exploration) -> String {
    if exploration.mode == Mode::Path {
        return PATH.to_owned();
    }
    open(exploration)
}

fn cycle(exploration: &Exploration, avoided: &[bool]) -> Option<(usize, Vec<usize>)> {
    let count = exploration.configuration.len();
    let mut state = vec![0u8; count];
    let mut stack = vec![(0usize, 0usize)];
    let mut trail = Vec::<usize>::new();
    state[0] = 1;
    while let Some(&mut (node, ref mut position)) = stack.last_mut() {
        let outgoing = &exploration.outgoing[node];
        let Some(&event) = outgoing.get(*position) else {
            state[node] = 2;
            stack.pop();
            trail.pop();
            continue;
        };
        *position += 1;
        let entry = &exploration.event[event];
        if !entry.supported || !avoided[entry.target] {
            continue;
        }
        match state[entry.target] {
            0 => {
                state[entry.target] = 1;
                trail.push(event);
                stack.push((entry.target, 0));
            }
            1 => {
                let mut route = trail.clone();
                route.push(event);
                return Some((entry.target, route));
            }
            _ => {}
        }
    }
    None
}

fn inevitable(exploration: &Exploration, matched: &[bool]) -> Evidence {
    if matched.first().copied().unwrap_or(false) {
        return Evidence::plain(Answer::Holds, "the start matches");
    }
    let avoided = matched
        .iter()
        .zip(&exploration.configuration)
        .map(|(&matched, configuration)| !matched && configuration.supported)
        .collect::<Vec<_>>();
    if let Some((entry, route)) = cycle(exploration, &avoided) {
        return Evidence {
            answer: Answer::Fails,
            witness: Some(entry),
            path: route,
            reason: format!("a run can cycle through s{entry} forever without a match"),
        };
    }
    if !exploration.closed {
        return Evidence::plain(Answer::Unknown, open(exploration));
    }
    let route = avoiding(exploration, &avoided);
    let end = exploration
        .leaf()
        .filter(|&index| route[index].is_some())
        .min_by_key(|&index| (route[index].as_ref().map_or(usize::MAX, Vec::len), index));
    let Some(index) = end else {
        return Evidence::plain(Answer::Holds, "every run reaches a match");
    };
    Evidence {
        answer: Answer::Fails,
        witness: Some(index),
        path: route[index].clone().unwrap_or_default(),
        reason: format!("a run ends at s{index} without a match"),
    }
}

fn avoiding(exploration: &Exploration, avoided: &[bool]) -> Vec<Option<Vec<usize>>> {
    let mut route = vec![None; exploration.configuration.len()];
    if !avoided.first().copied().unwrap_or(false) {
        return route;
    }
    route[0] = Some(Vec::new());
    let mut queue = std::collections::VecDeque::from([0]);
    while let Some(node) = queue.pop_front() {
        for &event in &exploration.outgoing[node] {
            let entry = &exploration.event[event];
            if !entry.supported || !avoided[entry.target] || route[entry.target].is_some() {
                continue;
            }
            let mut path = route[node].clone().unwrap_or_default();
            path.push(event);
            route[entry.target] = Some(path);
            queue.push_back(entry.target);
        }
    }
    route
}

fn exact(claim: &Claim, exploration: &Exploration) -> Result<Evidence, Failure> {
    if !matches!(claim.kind, Kind::Reach | Kind::Avoid) {
        return Err(Failure::new(
            Code::Claim,
            "only reach and avoid take an exact target",
        ));
    }
    if exploration.mode == Mode::Path {
        return Err(Failure::new(
            Code::Claim,
            "a direct path checks an exact target as its goal; explore in path mode with goal",
        ));
    }
    let target = crate::subject::lower("pattern", &claim.pattern, Code::Target)?;
    let verdict = exploration
        .verdict(&target, claim.preserve)
        .ok_or_else(|| Failure::new(Code::Claim, "this exploration cannot check exact targets"))?;
    let reach = claim.kind == Kind::Reach;
    Ok(match (verdict.outcome, verdict.witness) {
        (Outcome::Reached, Some(witness)) => Evidence::at(
            if reach { Answer::Holds } else { Answer::Fails },
            exploration,
            witness,
            format!("s{witness} is the target"),
        ),
        (Outcome::Unreachable, _) => Evidence::plain(
            if reach { Answer::Fails } else { Answer::Holds },
            "the exploration closed without the target",
        ),
        _ => Evidence::plain(Answer::Unknown, open(exploration)),
    })
}

fn pattern(claim: &Claim, exploration: &Exploration, item: &[Vec<pattern::Item>]) -> Evidence {
    let matched = (0..exploration.configuration.len())
        .map(|index| {
            exploration.configuration[index].supported
                && pattern::assign(item, exploration, index).is_some()
        })
        .collect::<Vec<_>>();
    let found = shallowest(
        exploration,
        supported(exploration).filter(|&index| matched[index]),
    );
    let missing = shallowest(
        exploration,
        supported(exploration).filter(|&index| !matched[index]),
    );
    let path = exploration.mode == Mode::Path;
    match claim.kind {
        Kind::Reach => match found {
            Some(index) => Evidence::at(
                Answer::Holds,
                exploration,
                index,
                format!("s{index} matches"),
            ),
            None if exploration.closed && !path => Evidence::plain(
                Answer::Fails,
                "no configuration matches, and the exploration closed",
            ),
            None => Evidence::plain(Answer::Unknown, unexplored(exploration)),
        },
        Kind::Avoid => match found {
            Some(index) => Evidence::at(
                Answer::Fails,
                exploration,
                index,
                format!("s{index} matches"),
            ),
            None if exploration.closed && !path => Evidence::plain(
                Answer::Holds,
                "no configuration matches, and the exploration closed",
            ),
            None => Evidence::plain(Answer::Unknown, unexplored(exploration)),
        },
        Kind::Always => match missing {
            Some(index) => Evidence::at(
                Answer::Fails,
                exploration,
                index,
                format!("s{index} does not match"),
            ),
            None if exploration.closed && !path => {
                Evidence::plain(Answer::Holds, "every configuration matches")
            }
            None => Evidence::plain(Answer::Unknown, unexplored(exploration)),
        },
        Kind::Inevitable if path => Evidence::plain(Answer::Unknown, PATH),
        Kind::Inevitable => inevitable(exploration, &matched),
        Kind::Outcome if path => Evidence::plain(Answer::Unknown, PATH),
        Kind::Outcome if !exploration.closed => Evidence::plain(
            Answer::Unknown,
            "end configurations are known once the exploration closes",
        ),
        Kind::Outcome => {
            let end = exploration.leaf().filter(|&index| !matched[index]);
            match shallowest(exploration, end) {
                Some(index) => Evidence::at(
                    Answer::Fails,
                    exploration,
                    index,
                    format!("s{index} ends without a match"),
                ),
                None => Evidence::plain(Answer::Holds, "every end configuration matches"),
            }
        }
    }
}

pub(crate) fn evaluate(claim: &Claim, exploration: &Exploration) -> Result<Verdict, Failure> {
    let evidence = if claim.exact {
        exact(claim, exploration)?
    } else {
        match Pattern::read(&claim.pattern)? {
            Pattern::Coherence(item) => pattern(claim, exploration, &item),
            Pattern::Rule(_) => {
                return Err(Failure::new(
                    Code::Claim,
                    "a claim is about configurations; write a coherence pattern such as False.Extra",
                ));
            }
        }
    };
    Ok(Verdict {
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
    })
}
