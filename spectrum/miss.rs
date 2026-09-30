use crate::claim;
use crate::configuration::{Coherence, Configuration, Occurrence, Value};
use crate::context::Context;
use crate::embedding;
use crate::exploration::Exploration;
use crate::extent::{Extent, Kind};
use crate::failure::{Code, Failure};
use crate::handle::Handle;
use crate::matching;
use crate::pattern::{self, Body, Item, Pattern, Region};
use crate::recording::{Mode, Recording};
use crate::render;
use frontend::source::Definition;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const LIMIT: usize = 3;

// Measuring a target pairs each of its parts with a place in every configuration, and no
// configuration holds more places than its budget allows, so a larger target only slows the answer.
const PART: usize = 256;

fn limit() -> usize {
    LIMIT
}

#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
#[schemars(
    description = "Explain why not: the configurations nearest a target and what they lack, or where a rule came closest to firing and what each of its inputs lacked."
)]
pub struct Request {
    #[serde(flatten)]
    pub recording: Recording,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "A pattern the program should reach, or with exact a complete configuration as Prism reads it."
    )]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[schemars(
        description = "Compare whole configurations as Prism does: coherences, live rules and open scopes."
    )]
    pub exact: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    #[schemars(
        description = "With exact, the target also lists every loaded root rule; without exact it is refused."
    )]
    pub preserve: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "A rule handle, such as r3, to explain why it does not fire.")]
    pub rule: Option<String>,
    #[serde(default = "limit")]
    #[schemars(description = "Configurations listed.")]
    pub limit: usize,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Near {
    pub(crate) handle: String,
    pub(crate) text: String,
    #[schemars(
        description = "Occurrences and rules missing plus those extra; 0 when the configuration matches."
    )]
    pub(crate) distance: usize,
    pub(crate) missing: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) extra: Vec<String>,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Lack {
    pub(crate) input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) coherence: Option<String>,
    pub(crate) missing: Vec<String>,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub(crate) struct Close {
    pub(crate) handle: String,
    pub(crate) text: String,
    pub(crate) missing: usize,
    pub(crate) lack: Vec<Lack>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    #[schemars(
        description = "The exploration recorded no events here, or a direct path took another event here, so the rule may still fire here."
    )]
    pub(crate) unexplored: bool,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub(crate) enum Miss {
    Target {
        target: String,
        exact: bool,
        near: Vec<Near>,
    },
    Rule {
        rule: String,
        fired: usize,
        #[schemars(
            description = "Configurations whose events the exploration recorded where the rule is live and did not fire."
        )]
        visible: usize,
        #[schemars(
            description = "Configurations where the rule is live but whose events the exploration did not record, or where a direct path took another event, so it may still fire there."
        )]
        unexplored: usize,
        near: Vec<Close>,
    },
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Answer {
    pub(crate) exploration: String,
    #[serde(flatten)]
    pub(crate) extent: Extent,
    #[serde(flatten)]
    pub(crate) miss: Miss,
}

struct Fit {
    matched: Vec<usize>,
    missing: Vec<String>,
}

fn describe(item: &Item) -> String {
    match item {
        Item::Atom(atom) => atom.clone(),
        Item::Rule(definition) => format!("({})", frontend::text::definition(definition)),
    }
}

fn fit(particle: &[Item], coherence: &Coherence, exploration: &Exploration) -> Fit {
    let mut matched = Vec::new();
    let mut missing = Vec::new();
    for item in particle {
        let found = coherence.occurrence.iter().find(|occurrence| {
            !matched.contains(&occurrence.id)
                && pattern::same(item, &occurrence.value, exploration.rule.as_slice())
        });
        match found {
            Some(occurrence) => matched.push(occurrence.id),
            None => missing.push(describe(item)),
        }
    }
    Fit { matched, missing }
}

fn assign(
    particle: &[Vec<Item>],
    coherence: &[&Coherence],
    exploration: &Exploration,
) -> Vec<Option<usize>> {
    let cost = particle
        .iter()
        .map(|part| {
            coherence
                .iter()
                .map(|entry| {
                    -i64::try_from(fit(part, entry, exploration).matched.len()).unwrap_or(i64::MAX)
                        - i64::from(part.is_empty())
                })
                .collect()
        })
        .collect::<Vec<Vec<i64>>>();
    matching::pair(&cost)
}

fn definition(occurrence: &Occurrence, exploration: &Exploration) -> Option<Definition> {
    let Value::Rule(index) = occurrence.value else {
        return None;
    };
    Some(exploration.rule[index].canonical.clone())
}

// What a target lacks and has extra in one configuration, and the frames and coherences its parts
// took, which no other part may take.
#[derive(Default)]
struct Tally {
    distance: usize,
    missing: Vec<String>,
    extra: Vec<String>,
    frame: Vec<usize>,
    coherence: Vec<usize>,
}

impl Tally {
    fn absorb(&mut self, other: Self) {
        self.distance += other.distance;
        self.missing.extend(other.missing);
        self.extra.extend(other.extra);
        self.frame.extend(other.frame);
        self.coherence.extend(other.coherence);
    }

    fn lack(&mut self, prefix: &str, particle: &[Item]) {
        self.distance += particle.len().max(1);
        if particle.is_empty() {
            self.missing.push(format!("{prefix}()"));
        }
        self.missing.extend(
            particle
                .iter()
                .map(|item| format!("{prefix}{}", describe(item))),
        );
    }
}

fn prefix(entry: &Configuration, frame: usize) -> String {
    if frame == 0 {
        return String::new();
    }
    format!("in {}: ", render::frame(entry, frame))
}

fn size(body: &Body) -> i64 {
    i64::try_from(body.size()).unwrap_or(i64::MAX)
}

// Each scope of the pattern takes the frame it fits best, and a scope that fits none is missing
// whole, so nested scopes are measured the way the configuration's own parts are. A scope found
// at any depth can take a frame another scope's parts took, and then it is missing whole too, so
// no frame serves two scopes.
fn nest(
    body: &Body,
    frame: &[usize],
    prefix: &str,
    inner: impl Fn(&Body, usize) -> Tally,
) -> Tally {
    let mut fit = body
        .scope
        .iter()
        .map(|scope| {
            frame
                .iter()
                .map(|&frame| {
                    let mut tally = inner(scope, frame);
                    tally.frame.push(frame);
                    tally
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let cost = body
        .scope
        .iter()
        .zip(&fit)
        .map(|(scope, fit)| {
            fit.iter()
                .map(|tally| {
                    i64::try_from(tally.distance)
                        .unwrap_or(i64::MAX)
                        .saturating_sub(size(scope))
                })
                .collect()
        })
        .collect::<Vec<Vec<i64>>>();
    let mut tally = Tally::default();
    for (row, column) in matching::pair(&cost).into_iter().enumerate() {
        if let Some(column) = column {
            let taken = std::mem::take(&mut fit[row][column]);
            if !taken.frame.iter().any(|frame| tally.frame.contains(frame)) {
                tally.absorb(taken);
                continue;
            }
        }
        let scope = &body.scope[row];
        tally.distance += scope.size();
        tally.missing.push(format!(
            "{prefix}{}",
            frontend::text::scope(&scope.program())
        ));
    }
    tally
}

fn contain(
    body: &Body,
    region: &Region,
    prefix: &str,
    exploration: &Exploration,
    configuration: usize,
) -> Tally {
    let entry = &exploration.configuration[configuration];
    let nested = nest(body, &region.frame, prefix, |scope, frame| {
        contain(
            scope,
            &Region::frame(entry, frame),
            &self::prefix(entry, frame),
            exploration,
            configuration,
        )
    });
    let candidate = region
        .coherence
        .iter()
        .copied()
        .filter(|index| !nested.coherence.contains(index))
        .collect::<Vec<_>>();
    let reference = candidate
        .iter()
        .map(|&index| &entry.coherence[index])
        .collect::<Vec<_>>();
    let mut tally = Tally::default();
    let choice = assign(&body.coherence, &reference, exploration);
    for (particle, choice) in body.coherence.iter().zip(choice) {
        let Some(position) = choice else {
            tally.lack(prefix, particle);
            continue;
        };
        tally.coherence.push(candidate[position]);
        let missing = fit(particle, reference[position], exploration).missing;
        tally.distance += missing.len();
        tally
            .missing
            .extend(missing.into_iter().map(|item| format!("{prefix}{item}")));
    }
    tally.absorb(nested);
    let mut live = region.rule.clone();
    for expected in &body.rule {
        match live
            .iter()
            .position(|&rule| exploration.rule[rule].canonical == *expected)
        {
            Some(position) => {
                live.swap_remove(position);
            }
            None => {
                tally.distance += 1;
                tally
                    .missing
                    .push(format!("{prefix}{}", frontend::text::definition(expected)));
            }
        }
    }
    tally
}

fn below(frame: &[usize], exploration: &Exploration, configuration: usize) -> Vec<usize> {
    let entry = &exploration.configuration[configuration];
    let mut result = frame.to_vec();
    let mut index = 0;
    while index < result.len() {
        let parent = result[index];
        result.extend(
            (0..entry.frame.len()).filter(|&child| entry.frame[child].parent == Some(parent)),
        );
        index += 1;
    }
    result
}

fn equal(body: &Body, frame: usize, exploration: &Exploration, configuration: usize) -> Tally {
    let entry = &exploration.configuration[configuration];
    let region = Region::opened(entry, frame);
    let prefix = prefix(entry, frame);
    let mut tally = Tally::default();
    let candidate = region
        .coherence
        .iter()
        .map(|&index| &entry.coherence[index])
        .collect::<Vec<_>>();
    let choice = assign(&body.coherence, &candidate, exploration);
    for (particle, choice) in body.coherence.iter().zip(&choice) {
        let Some(position) = *choice else {
            tally.lack(&prefix, particle);
            continue;
        };
        let fit = fit(particle, candidate[position], exploration);
        tally.distance += fit.missing.len();
        tally
            .missing
            .extend(fit.missing.iter().map(|item| format!("{prefix}{item}")));
        let surplus = candidate[position]
            .occurrence
            .iter()
            .filter(|occurrence| !fit.matched.contains(&occurrence.id))
            .map(|occurrence| {
                format!(
                    "{prefix}{}",
                    render::occurrence(&exploration.rule, occurrence)
                )
            })
            .collect::<Vec<_>>();
        tally.distance += surplus.len();
        tally.extra.extend(surplus);
    }
    for (position, coherence) in candidate.iter().enumerate() {
        if !choice.contains(&Some(position)) {
            tally.distance += coherence.occurrence.len().max(1);
            tally.extra.push(format!(
                "{prefix}{}",
                render::coherence(&exploration.rule, coherence)
            ));
        }
    }
    let nested = nest(body, &region.frame, &prefix, |scope, child| {
        equal(scope, child, exploration, configuration)
    });
    let unmatched = (0..entry.frame.len())
        .filter(|&child| entry.frame[child].parent == Some(frame) && !nested.frame.contains(&child))
        .collect::<Vec<_>>();
    tally.absorb(nested);
    let unmatched = below(&unmatched, exploration, configuration);
    for coherence in entry
        .coherence
        .iter()
        .filter(|coherence| unmatched.contains(&coherence.frame))
    {
        tally.distance += coherence.occurrence.len().max(1);
        tally.extra.push(format!(
            "{}{}",
            self::prefix(entry, coherence.frame),
            render::coherence(&exploration.rule, coherence)
        ));
    }
    let mut live = entry.frame[frame]
        .rule
        .iter()
        .filter_map(|occurrence| definition(occurrence, exploration).map(|rule| (rule, occurrence)))
        .collect::<Vec<_>>();
    for expected in &body.rule {
        match live.iter().position(|(rule, _)| rule == expected) {
            Some(position) => {
                live.remove(position);
            }
            None => {
                tally.distance += 1;
                tally
                    .missing
                    .push(format!("{prefix}{}", frontend::text::definition(expected)));
            }
        }
    }
    tally.distance += live.len();
    tally.extra.extend(live.iter().map(|(_, occurrence)| {
        format!(
            "{prefix}{}",
            render::occurrence(&exploration.rule, occurrence)
        )
    }));
    tally
}

// A configuration a pattern matches is at distance 0, as select and claims find it; any other is
// measured by the nearest assignment, which never uses one of its parts twice.
fn measure(
    body: &Body,
    exact: bool,
    exploration: &Exploration,
    index: usize,
) -> Result<(Near, usize), Failure> {
    let entry = &exploration.configuration[index];
    let tally = if exact {
        equal(body, 0, exploration, index)
    } else if embedding::assign(body, entry, exploration.rule.as_slice())?.is_some() {
        Tally::default()
    } else {
        contain(body, &Region::configuration(entry), "", exploration, index)
    };
    Ok((
        Near {
            handle: Handle::Configuration(index).to_string(),
            text: render::configuration(exploration, index),
            distance: tally.distance,
            missing: tally.missing,
            extra: tally.extra,
        },
        index,
    ))
}

fn depth(exploration: &Exploration, index: usize) -> usize {
    exploration.depth[index].unwrap_or(usize::MAX)
}

fn parts(body: &Body) -> usize {
    body.coherence.len()
        + body.rule.len()
        + body
            .scope
            .iter()
            .map(|scope| 1 + parts(scope))
            .sum::<usize>()
}

// A target read before anything is explored: a pattern, or with exact the configuration a program
// starts in, of at most PART parts.
fn read(request: &Request, text: &str) -> Result<Body, Failure> {
    let (body, code) = if request.exact {
        let target = crate::subject::lower("target", text, Code::Target)?;
        (Body::new(&target), Code::Target)
    } else {
        match Pattern::read(text)? {
            Pattern::Configuration(body) => (body, Code::Pattern),
            Pattern::Rule(_) => {
                return Err(Failure::new(
                    Code::Pattern,
                    "miss takes a pattern of coherences and scopes as its target; give a rule handle, such as r3, as its rule",
                ));
            }
        }
    };
    let count = parts(&body);
    if count > PART {
        return Err(Failure::new(
            code,
            format!(
                "miss measures targets of at most {PART} parts, and this one has {count}; ask select or check whether it matches, or measure a part of it"
            ),
        ));
    }
    Ok(body)
}

fn target(
    request: &Request,
    text: &str,
    mut body: Body,
    exploration: &Exploration,
) -> Result<Miss, Failure> {
    if request.preserve {
        let root = exploration
            .configuration
            .first()
            .and_then(|entry| entry.frame.first())
            .map(|frame| frame.rule.as_slice())
            .unwrap_or_default();
        body.rule.extend(
            root.iter()
                .filter_map(|occurrence| definition(occurrence, exploration)),
        );
    }
    let mut near = (0..exploration.configuration.len())
        .filter(|&index| exploration.configuration[index].supported)
        .map(|index| measure(&body, request.exact, exploration, index))
        .collect::<Result<Vec<_>, _>>()?;
    near.sort_by_key(|(entry, index)| (entry.distance, depth(exploration, *index), *index));
    Ok(Miss::Target {
        target: text.to_owned(),
        exact: request.exact,
        near: near
            .into_iter()
            .take(request.limit)
            .map(|(entry, _)| entry)
            .collect(),
    })
}

fn visible(exploration: &Exploration, configuration: usize, rule: usize) -> Vec<usize> {
    let entry = &exploration.configuration[configuration];
    let live = |frame: usize| {
        entry.frame[frame]
            .rule
            .iter()
            .any(|occurrence| occurrence.value == Value::Rule(rule))
    };
    (0..entry.frame.len())
        .filter(|&frame| {
            std::iter::successors(Some(frame), |&current| entry.frame[current].lexical).any(live)
        })
        .collect()
}

fn close(
    input: &[Vec<Item>],
    frame: &[usize],
    exploration: &Exploration,
    configuration: usize,
) -> Close {
    let candidate = exploration.configuration[configuration]
        .coherence
        .iter()
        .enumerate()
        .filter(|(_, coherence)| frame.contains(&coherence.frame))
        .collect::<Vec<_>>();
    let reference = candidate
        .iter()
        .map(|&(_, coherence)| coherence)
        .collect::<Vec<_>>();
    let choice = assign(input, &reference, exploration);
    let lack = input
        .iter()
        .zip(&choice)
        .map(|(particle, choice)| {
            let chosen = choice.map(|position| candidate[position]);
            Lack {
                input: if particle.is_empty() {
                    "()".to_owned()
                } else {
                    particle.iter().map(describe).collect::<Vec<_>>().join(".")
                },
                coherence: chosen
                    .map(|(world, _)| Handle::Coherence(configuration, world).to_string()),
                missing: chosen.map_or_else(
                    || particle.iter().map(describe).collect(),
                    |(_, coherence)| fit(particle, coherence, exploration).missing,
                ),
            }
        })
        .collect::<Vec<_>>();
    Close {
        handle: Handle::Configuration(configuration).to_string(),
        text: render::configuration(exploration, configuration),
        missing: lack.iter().map(|lack| lack.missing.len()).sum(),
        lack,
        unexplored: false,
    }
}

fn silent(exploration: &Exploration, configuration: usize, rule: usize) -> bool {
    exploration.configuration[configuration].supported
        && !exploration.outgoing[configuration].iter().any(|&event| {
            exploration.event[event].rule == rule && exploration.event[event].supported
        })
}

// Whether an exploration recorded a configuration's events: every one's once it closed, and while
// it is open those it recorded any event of; a direct path records only the event it took.
fn expanded(exploration: &Exploration, configuration: usize) -> bool {
    exploration.closed
        || (exploration.mode != Mode::Path && !exploration.outgoing[configuration].is_empty())
}

fn rule(request: &Request, text: &str, exploration: &Exploration) -> Result<Miss, Failure> {
    let handle = text.parse::<Handle>()?.check(exploration)?;
    let Handle::Rule(index) = handle else {
        return Err(Failure::new(
            Code::Handle,
            "rule takes a rule handle, such as r3",
        ));
    };
    let input = exploration.rule[index]
        .definition
        .input
        .iter()
        .map(|particle| particle.iter().map(pattern::item).collect())
        .collect::<Vec<Vec<Item>>>();
    let mut near = (0..exploration.configuration.len())
        .filter(|&configuration| silent(exploration, configuration, index))
        .filter_map(|configuration| {
            let frame = visible(exploration, configuration, index);
            (!frame.is_empty()).then(|| {
                (
                    Close {
                        unexplored: !expanded(exploration, configuration),
                        ..close(&input, &frame, exploration, configuration)
                    },
                    configuration,
                )
            })
        })
        .collect::<Vec<_>>();
    let unexplored = near.iter().filter(|(entry, _)| entry.unexplored).count();
    let live = near.len() - unexplored;
    near.sort_by_key(|(entry, configuration)| {
        (
            entry.missing,
            depth(exploration, *configuration),
            *configuration,
        )
    });
    Ok(Miss::Rule {
        rule: format!("{handle} {}", exploration.rule[index].text),
        fired: exploration.firing(index).count(),
        visible: live,
        unexplored,
        near: near
            .into_iter()
            .take(request.limit)
            .map(|(entry, _)| entry)
            .collect(),
    })
}

// What a miss explains, checked before anything is explored: a target, or a rule.
enum Question<'request> {
    Target(&'request str, Body),
    Rule(&'request str),
}

fn question(request: &Request) -> Result<Question<'_>, Failure> {
    match (&request.target, &request.rule) {
        (Some(_), Some(_)) => Err(Failure::new(
            Code::Request,
            "explain a target or a rule, not both",
        )),
        (None, None) => Err(Failure::new(
            Code::Request,
            "name what to explain: a target (--target, or target), or a rule handle such as r3 (after the files, or rule)",
        )),
        (Some(text), None) => {
            claim::preserve(request.exact, request.preserve)?;
            Ok(Question::Target(text, read(request, text)?))
        }
        (None, Some(_)) if request.exact || request.preserve => Err(Failure::new(
            Code::Request,
            "exact and preserve read a target, and a rule takes neither",
        )),
        (None, Some(text)) => Ok(Question::Rule(text)),
    }
}

pub(crate) fn answer(request: &Request, context: &mut Context<'_>) -> Result<Answer, Failure> {
    let question = question(request)?;
    let exploration = context.exploration(&request.recording)?;
    let miss = match question {
        Question::Target(text, body) => target(request, text, body, &exploration)?,
        Question::Rule(text) => rule(request, text, &exploration)?,
    };
    Ok(Answer {
        exploration: exploration.name(),
        extent: exploration.extent(),
        miss,
    })
}

impl Answer {
    pub(crate) fn text(&self) -> String {
        let mut line = Vec::new();
        let extent = self.extent;
        let state = extent.name();
        let width = match &self.miss {
            Miss::Target { near, .. } => near.iter().map(|entry| entry.text.chars().count()).max(),
            Miss::Rule { near, .. } => near.iter().map(|entry| entry.text.chars().count()).max(),
        }
        .unwrap_or(0);
        match &self.miss {
            Miss::Target {
                target,
                exact,
                near,
            } => {
                let exact = if *exact { " exactly" } else { "" };
                let lead = if near.first().is_some_and(|entry| entry.distance == 0) {
                    "reaches"
                } else {
                    "nearest to"
                };
                line.push(format!(
                    "{} · {state} · {lead} {target}{exact}",
                    self.exploration
                ));
                for entry in near {
                    let mut difference = Vec::new();
                    if !entry.missing.is_empty() {
                        difference.push(format!("lacks {}", entry.missing.join(", ")));
                    }
                    if !entry.extra.is_empty() {
                        difference.push(format!("extra {}", entry.extra.join(", ")));
                    }
                    if difference.is_empty() {
                        difference.push("matches".to_owned());
                    }
                    line.push(format!(
                        "{:<5} {:<width$}   {}",
                        entry.handle,
                        entry.text,
                        difference.join("; ")
                    ));
                }
            }
            Miss::Rule {
                rule,
                fired,
                visible,
                unexplored,
                near,
            } => {
                let fired = extent.firing(*fired);
                let live = match extent.kind() {
                    Kind::Closed => format!(
                        "live in {} where it does not fire",
                        render::count(*visible, "configuration")
                    ),
                    Kind::Open => format!(
                        "live in {} where it has not fired yet, and in {} unexplored",
                        render::count(*visible, "explored configuration"),
                        unexplored
                    ),
                    Kind::Path => format!(
                        "live in {} of the path, which did not take it",
                        render::count(*unexplored, "configuration")
                    ),
                };
                line.push(format!("{rule}   {fired} · {state} · {live}"));
                for entry in near {
                    let lack = entry
                        .lack
                        .iter()
                        .map(|lack| {
                            let place = lack.coherence.as_deref().unwrap_or("no coherence");
                            if lack.missing.is_empty() {
                                format!("[{}] matched by {place}", lack.input)
                            } else {
                                format!(
                                    "[{}] lacks {} in {place}",
                                    lack.input,
                                    lack.missing.join(", ")
                                )
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("; ");
                    let note = if entry.unexplored && extent.kind() == Kind::Open {
                        "   unexplored"
                    } else {
                        ""
                    };
                    line.push(format!(
                        "{:<5} {:<width$}   {lack}{note}",
                        entry.handle, entry.text
                    ));
                }
            }
        }
        line.join("\n")
    }
}
