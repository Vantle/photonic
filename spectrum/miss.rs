use crate::context::Context;
use crate::exploration::{Coherence, Exploration, Occurrence, Value};
use crate::failure::{Code, Failure};
use crate::handle::Handle;
use crate::matching;
use crate::pattern::{self, Item, Pattern};
use crate::recording::Recording;
use crate::render;
use photonic::source::Definition;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

fn limit() -> usize {
    3
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
pub struct Near {
    pub handle: String,
    pub text: String,
    #[schemars(
        description = "Occurrences and rules missing plus those extra; 0 when the configuration matches."
    )]
    pub distance: usize,
    pub missing: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extra: Vec<String>,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Lack {
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coherence: Option<String>,
    pub missing: Vec<String>,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
pub struct Close {
    pub handle: String,
    pub text: String,
    pub missing: usize,
    pub lack: Vec<Lack>,
}

#[derive(Clone, Debug, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Miss {
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
    pub exploration: String,
    pub complete: bool,
    #[serde(flatten)]
    pub miss: Miss,
}

struct Fit {
    matched: Vec<usize>,
    missing: Vec<String>,
}

fn describe(item: &Item) -> String {
    match item {
        Item::Atom(atom) => atom.clone(),
        Item::Rule(definition) => format!("({})", photonic::text::definition(definition)),
    }
}

fn fit(particle: &[Item], coherence: &Coherence, exploration: &Exploration) -> Fit {
    let mut matched = Vec::new();
    let mut missing = Vec::new();
    for item in particle {
        let found = coherence.occurrence.iter().find(|occurrence| {
            !matched.contains(&occurrence.id) && pattern::same(item, &occurrence.value, exploration)
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

fn bare(definition: &Definition) -> Definition {
    Definition {
        name: String::new(),
        ..definition.clone()
    }
    .canonical()
}

fn definition(occurrence: &Occurrence, exploration: &Exploration) -> Option<Definition> {
    let Value::Rule(index) = occurrence.value else {
        return None;
    };
    Some(exploration.rule[index].canonical.clone())
}

struct Exact {
    rule: Vec<Definition>,
}

fn measure(
    particle: &[Vec<Item>],
    exact: Option<&Exact>,
    exploration: &Exploration,
    index: usize,
) -> (Near, usize) {
    let entry = &exploration.configuration[index];
    let candidate = entry
        .coherence
        .iter()
        .filter(|coherence| exact.is_none() || coherence.frame == 0)
        .collect::<Vec<_>>();
    let choice = assign(particle, &candidate, exploration);
    let mut missing = Vec::new();
    let mut extra = Vec::new();
    let mut distance = 0;
    for (part, choice) in particle.iter().zip(&choice) {
        let Some(position) = *choice else {
            distance += part.len();
            missing.extend(part.iter().map(describe));
            continue;
        };
        let fit = fit(part, candidate[position], exploration);
        distance += fit.missing.len();
        missing.extend(fit.missing);
        if exact.is_none() {
            continue;
        }
        let surplus = candidate[position]
            .occurrence
            .iter()
            .filter(|occurrence| !fit.matched.contains(&occurrence.id))
            .map(|occurrence| render::occurrence(exploration, occurrence))
            .collect::<Vec<_>>();
        distance += surplus.len();
        extra.extend(surplus);
    }
    if let Some(exact) = exact {
        for (position, coherence) in candidate.iter().enumerate() {
            if !choice.contains(&Some(position)) {
                distance += coherence.occurrence.len().max(1);
                extra.push(render::coherence(exploration, coherence));
            }
        }
        for coherence in entry
            .coherence
            .iter()
            .filter(|coherence| coherence.frame != 0)
        {
            distance += coherence.occurrence.len().max(1);
            extra.push(format!(
                "in f{}: {}",
                coherence.frame,
                render::coherence(exploration, coherence)
            ));
        }
        let mut live = entry
            .frame
            .first()
            .map(|frame| frame.rule.as_slice())
            .unwrap_or_default()
            .iter()
            .filter_map(|occurrence| {
                definition(occurrence, exploration).map(|rule| (rule, occurrence))
            })
            .collect::<Vec<_>>();
        for expected in &exact.rule {
            match live.iter().position(|(rule, _)| rule == expected) {
                Some(position) => {
                    live.remove(position);
                }
                None => {
                    distance += 1;
                    missing.push(format!("({})", photonic::text::definition(expected)));
                }
            }
        }
        distance += live.len();
        extra.extend(
            live.iter()
                .map(|(_, occurrence)| render::occurrence(exploration, occurrence)),
        );
    }
    (
        Near {
            handle: Handle::Configuration(index).to_string(),
            text: render::configuration(exploration, index),
            distance,
            missing,
            extra,
        },
        index,
    )
}

fn depth(exploration: &Exploration, index: usize) -> usize {
    exploration.depth[index].unwrap_or(usize::MAX)
}

fn target(request: &Request, text: &str, exploration: &Exploration) -> Result<Miss, Failure> {
    let (particle, exact) = if request.exact {
        let target = crate::subject::lower("target", text, Code::Target)?;
        let mut rule = target.rule.iter().map(bare).collect::<Vec<_>>();
        if request.preserve {
            let root = exploration
                .configuration
                .first()
                .and_then(|entry| entry.frame.first())
                .map(|frame| frame.rule.as_slice())
                .unwrap_or_default();
            rule.extend(
                root.iter()
                    .filter_map(|occurrence| definition(occurrence, exploration)),
            );
        }
        let particle = target
            .initial
            .iter()
            .map(|particle| particle.iter().map(pattern::item).collect())
            .collect::<Vec<Vec<Item>>>();
        (particle, Some(Exact { rule }))
    } else {
        if request.preserve {
            return Err(Failure::new(
                Code::Request,
                "preserve adds rules to an exact target; set exact",
            ));
        }
        match Pattern::read(text)? {
            Pattern::Coherence(item) => (item, None),
            Pattern::Rule(_) => {
                return Err(Failure::new(
                    Code::Pattern,
                    "miss takes a coherence pattern as target; give a rule as rule",
                ));
            }
        }
    };
    let mut near = (0..exploration.configuration.len())
        .filter(|&index| exploration.configuration[index].supported)
        .map(|index| measure(&particle, exact.as_ref(), exploration, index))
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

pub fn answer(request: &Request, context: &mut Context<'_>) -> Result<Answer, Failure> {
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
    pub fn text(&self) -> String {
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
