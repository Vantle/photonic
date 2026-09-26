use crate::configuration::{Coherence, Occurrence, Value};
use crate::context::Context;
use crate::exploration::Exploration;
use crate::failure::{Code, Failure};
use crate::handle::Handle;
use crate::matching;
use crate::pattern::{self, Body, Item, Pattern, Region};
use crate::recording::Recording;
use crate::render;
use frontend::source::Definition;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const LIMIT: usize = 3;

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
    #[schemars(description = "With exact, the target also lists every loaded root rule.")]
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
        visible: usize,
        near: Vec<Close>,
    },
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Answer {
    pub(crate) exploration: String,
    pub(crate) complete: bool,
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
    let width = coherence.len() + particle.len();
    let cost = particle
        .iter()
        .map(|part| {
            (0..width)
                .map(|column| {
                    coherence.get(column).map_or(0, |entry| {
                        -i64::try_from(fit(part, entry, exploration).matched.len())
                            .unwrap_or(i64::MAX)
                            - i64::from(part.is_empty())
                    })
                })
                .collect()
        })
        .collect::<Vec<Vec<i64>>>();
    matching::cheapest(&cost)
        .into_iter()
        .map(|column| (column < coherence.len()).then_some(column))
        .collect()
}

fn definition(occurrence: &Occurrence, exploration: &Exploration) -> Option<Definition> {
    let Value::Rule(index) = occurrence.value else {
        return None;
    };
    Some(exploration.rule[index].canonical.clone())
}

#[derive(Default)]
struct Tally {
    distance: usize,
    missing: Vec<String>,
    extra: Vec<String>,
}

impl Tally {
    fn absorb(&mut self, other: Self) {
        self.distance += other.distance;
        self.missing.extend(other.missing);
        self.extra.extend(other.extra);
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

fn prefix(frame: usize) -> String {
    if frame == 0 {
        return String::new();
    }
    format!("in f{frame}: ")
}

// Each scope of the pattern takes the frame it fits best, and a scope that fits none is missing
// whole, so nested scopes are measured the way the configuration's own parts are.
fn nest(
    body: &Body,
    frame: &[usize],
    prefix: &str,
    inner: impl Fn(&Body, usize) -> Tally,
) -> (Tally, Vec<usize>) {
    let mut fit = body
        .scope
        .iter()
        .map(|scope| {
            frame
                .iter()
                .map(|&frame| inner(scope, frame))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let width = frame.len() + body.scope.len();
    let cost = body
        .scope
        .iter()
        .zip(&fit)
        .map(|(scope, fit)| {
            (0..width)
                .map(|column| {
                    let distance = fit.get(column).map_or(scope.size(), |tally| tally.distance);
                    i64::try_from(distance).unwrap_or(i64::MAX)
                })
                .collect()
        })
        .collect::<Vec<Vec<i64>>>();
    let mut tally = Tally::default();
    let mut used = Vec::new();
    for (row, column) in matching::cheapest(&cost).into_iter().enumerate() {
        if column < frame.len() {
            tally.absorb(std::mem::take(&mut fit[row][column]));
            used.push(frame[column]);
            continue;
        }
        let scope = &body.scope[row];
        tally.distance += scope.size();
        tally.missing.push(format!(
            "{prefix}{}",
            frontend::text::scope(&scope.program())
        ));
    }
    (tally, used)
}

fn contain(
    body: &Body,
    region: &Region,
    prefix: &str,
    exploration: &Exploration,
    configuration: usize,
) -> Tally {
    let entry = &exploration.configuration[configuration];
    let mut tally = Tally::default();
    let candidate = region
        .coherence
        .iter()
        .map(|&index| &entry.coherence[index])
        .collect::<Vec<_>>();
    let choice = assign(&body.coherence, &candidate, exploration);
    for (particle, choice) in body.coherence.iter().zip(choice) {
        let Some(position) = choice else {
            tally.lack(prefix, particle);
            continue;
        };
        let missing = fit(particle, candidate[position], exploration).missing;
        tally.distance += missing.len();
        tally
            .missing
            .extend(missing.into_iter().map(|item| format!("{prefix}{item}")));
    }
    let (nested, _) = nest(body, &region.frame, prefix, |scope, frame| {
        let region = Region::frame(&exploration.configuration[configuration], frame);
        contain(
            scope,
            &region,
            &self::prefix(frame),
            exploration,
            configuration,
        )
    });
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
                tally.missing.push(format!(
                    "{prefix}({})",
                    frontend::text::definition(expected)
                ));
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
    let prefix = prefix(frame);
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
            .map(|occurrence| format!("{prefix}{}", render::occurrence(exploration, occurrence)))
            .collect::<Vec<_>>();
        tally.distance += surplus.len();
        tally.extra.extend(surplus);
    }
    for (position, coherence) in candidate.iter().enumerate() {
        if !choice.contains(&Some(position)) {
            tally.distance += coherence.occurrence.len().max(1);
            tally.extra.push(format!(
                "{prefix}{}",
                render::coherence(exploration, coherence)
            ));
        }
    }
    let (nested, used) = nest(body, &region.frame, &prefix, |scope, child| {
        equal(scope, child, exploration, configuration)
    });
    tally.absorb(nested);
    let unmatched = (0..entry.frame.len())
        .filter(|&child| entry.frame[child].parent == Some(frame) && !used.contains(&child))
        .collect::<Vec<_>>();
    let unmatched = below(&unmatched, exploration, configuration);
    for coherence in entry
        .coherence
        .iter()
        .filter(|coherence| unmatched.contains(&coherence.frame))
    {
        tally.distance += coherence.occurrence.len().max(1);
        tally.extra.push(format!(
            "in f{}: {}",
            coherence.frame,
            render::coherence(exploration, coherence)
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
                tally.missing.push(format!(
                    "{prefix}({})",
                    frontend::text::definition(expected)
                ));
            }
        }
    }
    tally.distance += live.len();
    tally.extra.extend(
        live.iter().map(|(_, occurrence)| {
            format!("{prefix}{}", render::occurrence(exploration, occurrence))
        }),
    );
    tally
}

fn measure(body: &Body, exact: bool, exploration: &Exploration, index: usize) -> (Near, usize) {
    let tally = if exact {
        equal(body, 0, exploration, index)
    } else {
        let region = Region::configuration(&exploration.configuration[index]);
        contain(body, &region, "", exploration, index)
    };
    (
        Near {
            handle: Handle::Configuration(index).to_string(),
            text: render::configuration(exploration, index),
            distance: tally.distance,
            missing: tally.missing,
            extra: tally.extra,
        },
        index,
    )
}

fn depth(exploration: &Exploration, index: usize) -> usize {
    exploration.depth[index].unwrap_or(usize::MAX)
}

fn target(request: &Request, text: &str, exploration: &Exploration) -> Result<Miss, Failure> {
    let body = if request.exact {
        let target = crate::subject::lower("target", text, Code::Target)?;
        let mut body = Body::new(&target);
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
        body
    } else {
        if request.preserve {
            return Err(Failure::new(
                Code::Request,
                "preserve adds rules to an exact target; set exact",
            ));
        }
        match Pattern::read(text)? {
            Pattern::Configuration(body) => body,
            Pattern::Rule(_) => {
                return Err(Failure::new(
                    Code::Pattern,
                    "miss takes a pattern of coherences and scopes as target; give a rule as rule",
                ));
            }
        }
    };
    let mut near = (0..exploration.configuration.len())
        .filter(|&index| exploration.configuration[index].supported)
        .map(|index| measure(&body, request.exact, exploration, index))
        .collect::<Vec<_>>();
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
                input: particle.iter().map(describe).collect::<Vec<_>>().join("."),
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
    }
}

fn silent(exploration: &Exploration, configuration: usize, rule: usize) -> bool {
    exploration.configuration[configuration].supported
        && !exploration.outgoing[configuration].iter().any(|&event| {
            exploration.event[event].rule == rule && exploration.event[event].supported
        })
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
                    close(&input, &frame, exploration, configuration),
                    configuration,
                )
            })
        })
        .collect::<Vec<_>>();
    let live = near.len();
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
        near: near
            .into_iter()
            .take(request.limit)
            .map(|(entry, _)| entry)
            .collect(),
    })
}

pub(crate) fn answer(request: &Request, context: &mut Context<'_>) -> Result<Answer, Failure> {
    let exploration = context.exploration(&request.recording)?;
    let miss = match (&request.target, &request.rule) {
        (Some(text), None) => target(request, text, &exploration)?,
        (None, Some(text)) => rule(request, text, &exploration)?,
        _ => {
            return Err(Failure::new(Code::Request, "give target or rule, not both"));
        }
    };
    Ok(Answer {
        exploration: exploration.name(),
        complete: exploration.closed,
        miss,
    })
}

impl Answer {
    pub(crate) fn text(&self) -> String {
        let mut line = Vec::new();
        let state = if self.complete { "closed" } else { "open" };
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
                near,
            } => {
                let fired = if *fired == 0 {
                    "never fires".to_owned()
                } else {
                    format!("fires {}", render::count(*fired, "time"))
                };
                line.push(format!(
                    "{rule}   {fired} · {state} · live in {} where it does not fire",
                    render::count(*visible, "configuration")
                ));
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
                    line.push(format!(
                        "{:<5} {:<width$}   {lack}",
                        entry.handle, entry.text
                    ));
                }
            }
        }
        line.join("\n")
    }
}
