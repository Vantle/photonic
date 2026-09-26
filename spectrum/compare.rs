use crate::claim::{self, Claim};
use crate::context::Context;
use crate::exploration::{Exploration, Occurrence, Opener};
use crate::failure::{Code, Failure};
use crate::handle::Handle;
use crate::recording::Recording;
use crate::render;
use code::canonical::{Exhausted, Key};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

const BUDGET: usize = 100_000;

pub const LIMIT: usize = 12;

fn limit() -> usize {
    LIMIT
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(
    description = "Compare two programs or explorations by behavior: the configurations each reaches, compared by their coherences and held occurrences up to occurrence identity; the events, compared by rule, so edited rules show there; and each claim's answer on both."
)]
pub struct Request {
    pub left: Recording,
    pub right: Recording,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claim: Vec<Claim>,
    #[serde(default = "limit")]
    #[schemars(description = "Differences listed in each direction; the rest are counted.")]
    pub limit: usize,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Side {
    pub(crate) exploration: String,
    pub(crate) complete: bool,
    pub(crate) configuration: usize,
    pub(crate) event: usize,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Entry {
    pub(crate) handle: String,
    pub(crate) text: String,
    pub(crate) path: Vec<String>,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Group {
    pub(crate) rule: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub(crate) inferred: bool,
    pub(crate) source: String,
    pub(crate) target: String,
    pub(crate) count: usize,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Change<Item> {
    #[schemars(
        description = "What the right side has and the left does not; handles name the right side."
    )]
    pub(crate) gained: Vec<Item>,
    #[schemars(
        description = "What the left side has and the right does not; handles name the left side."
    )]
    pub(crate) lost: Vec<Item>,
    #[schemars(description = "Differences beyond those listed.")]
    pub(crate) more: usize,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Pair {
    pub(crate) claim: Claim,
    pub(crate) left: claim::Answer,
    pub(crate) right: claim::Answer,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Answer {
    pub(crate) left: Side,
    pub(crate) right: Side,
    pub(crate) configuration: Change<Entry>,
    pub(crate) event: Change<Group>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) claim: Vec<Pair>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct Element {
    place: String,
    text: String,
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct Transition {
    rule: String,
    inferred: bool,
    source: Key<Element>,
    target: Key<Element>,
}

struct Index {
    configuration: BTreeMap<Key<Element>, usize>,
    event: BTreeMap<Transition, Vec<usize>>,
}

fn label(exploration: &Exploration, configuration: usize, frame: usize) -> String {
    match exploration.configuration[configuration].frame[frame].opener {
        Some(Opener::Rule(rule)) => render::input(exploration, rule),
        Some(Opener::Program) => "scope".to_owned(),
        None => "root".to_owned(),
    }
}

fn group(exploration: &Exploration, place: &str, occurrence: &[Occurrence]) -> Vec<(u32, Element)> {
    occurrence
        .iter()
        .map(|occurrence| {
            (
                u32::try_from(occurrence.id).unwrap_or(u32::MAX),
                Element {
                    place: place.to_owned(),
                    text: render::occurrence(exploration, occurrence),
                },
            )
        })
        .collect()
}

fn key(exploration: &Exploration, configuration: usize) -> Result<Key<Element>, Exhausted> {
    let entry = &exploration.configuration[configuration];
    let world = entry.coherence.iter().map(|coherence| {
        group(
            exploration,
            &label(exploration, configuration, coherence.frame),
            &coherence.occurrence,
        )
    });
    let held = entry.frame.iter().enumerate().map(|(position, frame)| {
        let place = label(exploration, configuration, position);
        group(exploration, &format!("{place} holds"), &frame.held)
    });
    code::canonical::key(&world.chain(held).collect::<Vec<_>>(), BUDGET)
}

fn index(exploration: &Exploration) -> Result<Index, Failure> {
    let key = (0..exploration.configuration.len())
        .map(|configuration| key(exploration, configuration))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| {
            Failure::new(
                Code::Exploration,
                "a configuration is too symmetric to compare within the budget",
            )
        })?;
    let mut configuration = BTreeMap::new();
    for (position, entry) in key.iter().enumerate() {
        if exploration.configuration[position].supported {
            configuration.entry(entry.clone()).or_insert(position);
        }
    }
    let mut event = BTreeMap::<_, Vec<usize>>::new();
    for (position, entry) in exploration.event.iter().enumerate() {
        if !entry.supported {
            continue;
        }
        event
            .entry(Transition {
                rule: render::brief(exploration, entry.rule),
                inferred: exploration.inferred(position),
                source: key[entry.source].clone(),
                target: key[entry.target].clone(),
            })
            .or_default()
            .push(position);
    }
    Ok(Index {
        configuration,
        event,
    })
}

fn side(exploration: &Exploration, index: &Index) -> Side {
    Side {
        exploration: exploration.name(),
        complete: exploration.settled(),
        configuration: index.configuration.len(),
        event: index.event.values().map(Vec::len).sum(),
    }
}

fn entry(exploration: &Exploration, configuration: usize) -> Entry {
    Entry {
        handle: Handle::Configuration(configuration).to_string(),
        text: render::configuration(exploration, configuration),
        path: exploration
            .path(configuration)
            .unwrap_or_default()
            .iter()
            .map(|&event| Handle::Event(event).to_string())
            .collect(),
    }
}

fn beyond(exploration: &Exploration, index: &Index, other: &Index) -> (Vec<Entry>, Vec<Group>) {
    let configuration = index
        .configuration
        .iter()
        .filter(|(key, _)| !other.configuration.contains_key(*key))
        .map(|(_, &position)| entry(exploration, position))
        .collect();
    let event = index
        .event
        .iter()
        .filter_map(|(key, member)| {
            let count = member
                .len()
                .saturating_sub(other.event.get(key).map_or(0, Vec::len));
            let &first = member.first()?;
            let entry = &exploration.event[first];
            (count > 0).then(|| Group {
                rule: key.rule.clone(),
                inferred: key.inferred,
                source: render::configuration(exploration, entry.source),
                target: render::configuration(exploration, entry.target),
                count,
            })
        })
        .collect();
    (configuration, event)
}

fn change<Item>(gained: Vec<Item>, lost: Vec<Item>, limit: usize) -> Change<Item> {
    let more = gained.len().saturating_sub(limit) + lost.len().saturating_sub(limit);
    Change {
        gained: gained.into_iter().take(limit).collect(),
        lost: lost.into_iter().take(limit).collect(),
        more,
    }
}

pub(crate) fn answer(request: &Request, context: &mut Context<'_>) -> Result<Answer, Failure> {
    let left = context.exploration(&request.left)?;
    let right = context.exploration(&request.right)?;
    let (before, after) = (index(&left)?, index(&right)?);
    let (lost, removed) = beyond(&left, &before, &after);
    let (gained, added) = beyond(&right, &after, &before);
    let claim = request
        .claim
        .iter()
        .map(|claim| {
            Ok(Pair {
                claim: claim.clone(),
                left: claim::evaluate(claim, &left)?.answer,
                right: claim::evaluate(claim, &right)?.answer,
            })
        })
        .collect::<Result<Vec<_>, Failure>>()?;
    Ok(Answer {
        left: side(&left, &before),
        right: side(&right, &after),
        configuration: change(gained, lost, request.limit),
        event: change(added, removed, request.limit),
        claim,
    })
}

impl Side {
    fn name(&self) -> String {
        if self.complete {
            return self.exploration.clone();
        }
        format!("{} open", self.exploration)
    }
}

impl<Item> Change<Item> {
    pub(crate) fn same(&self) -> bool {
        self.gained.is_empty() && self.lost.is_empty() && self.more == 0
    }
}

impl Answer {
    pub(crate) fn passed(&self) -> bool {
        self.left.complete
            && self.right.complete
            && self.configuration.same()
            && self.event.same()
            && self
                .claim
                .iter()
                .all(|pair| pair.left == pair.right && pair.left != claim::Answer::Unknown)
    }

    pub(crate) fn text(&self) -> String {
        let mut line = vec![format!(
            "compare {} {}",
            self.left.name(),
            self.right.name()
        )];
        if self.configuration.same() {
            line.push(format!(
                "configurations   identical · {}",
                self.left.configuration
            ));
        } else {
            line.push(format!(
                "configurations   {} → {}",
                self.left.configuration, self.right.configuration
            ));
        }
        let width = self
            .configuration
            .gained
            .iter()
            .chain(&self.configuration.lost)
            .map(|entry| entry.text.chars().count())
            .max()
            .unwrap_or(0);
        for entry in &self.configuration.gained {
            let path = if entry.path.is_empty() {
                String::new()
            } else {
                format!("   by {}", entry.path.join(" "))
            };
            line.push(format!(
                "  + {:<width$}   {}{path}",
                entry.text, entry.handle
            ));
        }
        for entry in &self.configuration.lost {
            line.push(format!(
                "  − {:<width$}   {} on the left",
                entry.text, entry.handle
            ));
        }
        if self.configuration.more > 0 {
            line.push(format!(
                "  and {}",
                render::count(self.configuration.more, "more configuration")
            ));
        }
        if self.event.same() {
            line.push(format!("events           identical · {}", self.left.event));
        } else {
            line.push(format!(
                "events           {} → {}",
                self.left.event, self.right.event
            ));
        }
        let width = self
            .event
            .gained
            .iter()
            .chain(&self.event.lost)
            .map(|group| group.rule.chars().count())
            .max()
            .unwrap_or(0);
        let signed = self
            .event
            .gained
            .iter()
            .map(|group| ("+", group))
            .chain(self.event.lost.iter().map(|group| ("−", group)));
        for (sign, group) in signed {
            let count = if group.count > 1 {
                format!("   ×{}", group.count)
            } else {
                String::new()
            };
            let inferred = if group.inferred { "   inferred" } else { "" };
            line.push(format!(
                "  {sign} {:<width$}   {} → {}{count}{inferred}",
                group.rule, group.source, group.target
            ));
        }
        if self.event.more > 0 {
            line.push(format!(
                "  and {}",
                render::count(self.event.more, "more event difference")
            ));
        }
        for pair in &self.claim {
            let exact = if pair.claim.exact { " exactly" } else { "" };
            line.push(format!(
                "{} {}{exact}   {} → {}",
                render::name(pair.claim.kind),
                pair.claim.pattern,
                render::name(pair.left),
                render::name(pair.right)
            ));
        }
        line.join("\n")
    }
}
