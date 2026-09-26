use crate::configuration::{Coherence, Configuration, Opener, Value};
use crate::exploration::Rule;
use crate::failure::{Code, Failure};
use frontend::source::{self, Definition, Program};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Item {
    Atom(String),
    Rule(Definition),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Body {
    pub coherence: Vec<Vec<Item>>,
    pub rule: Vec<Definition>,
    pub scope: Vec<Self>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Pattern {
    Configuration(Body),
    Rule(Vec<Definition>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Match {
    pub coherence: usize,
    pub occurrence: Vec<usize>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Embedding {
    pub coherence: Vec<Match>,
    pub frame: Vec<usize>,
}

pub struct Region {
    pub coherence: Vec<usize>,
    pub frame: Vec<usize>,
    pub rule: Vec<usize>,
}

pub trait Canon {
    fn canonical(&self, rule: usize) -> Option<&Definition>;
}

impl Canon for [Rule] {
    fn canonical(&self, rule: usize) -> Option<&Definition> {
        self.get(rule).map(|entry| &entry.canonical)
    }
}

impl Canon for [Option<Definition>] {
    fn canonical(&self, rule: usize) -> Option<&Definition> {
        self.get(rule)?.as_ref()
    }
}

pub fn item(value: &source::Value) -> Item {
    match value {
        source::Value::Atom(atom) => Item::Atom(atom.clone()),
        source::Value::Rule { rule } => Item::Rule(rule.canonical()),
    }
}

fn value(item: &Item) -> source::Value {
    match item {
        Item::Atom(atom) => source::Value::Atom(atom.clone()),
        Item::Rule(definition) => source::Value::Rule {
            rule: Box::new(definition.clone()),
        },
    }
}

impl Body {
    pub fn new(program: &Program) -> Self {
        Self {
            coherence: program
                .initial
                .iter()
                .map(|particle| particle.iter().map(item).collect())
                .collect(),
            rule: program.rule.iter().map(Definition::canonical).collect(),
            scope: program.scope.iter().map(Self::new).collect(),
        }
    }

    pub fn program(&self) -> Program {
        Program {
            initial: self
                .coherence
                .iter()
                .map(|particle| particle.iter().map(value).collect())
                .collect(),
            rule: self.rule.clone(),
            scope: self.scope.iter().map(Self::program).collect(),
        }
    }

    pub fn size(&self) -> usize {
        self.coherence
            .iter()
            .map(|particle| particle.len().max(1))
            .sum::<usize>()
            + self.rule.len()
            + self.scope.iter().map(Self::size).sum::<usize>()
    }
}

impl Pattern {
    pub fn read(text: &str) -> Result<Self, Failure> {
        if text.trim().is_empty() {
            return Err(Failure::new(
                Code::Pattern,
                "write a pattern, such as B.X, B, C, ().([A] B), (K, [K] L) or [B, C] D",
            ));
        }
        let program = frontend::lowering::parse(text)
            .map_err(|error| Failure::located(Code::Pattern, &error, "pattern", text))?;
        Self::new(&program).map_err(|message| Failure::new(Code::Pattern, message))
    }

    pub fn new(program: &Program) -> Result<Self, &'static str> {
        if program.rule.is_empty() {
            return Ok(Self::Configuration(Body::new(program)));
        }
        if !program.initial.is_empty() || !program.scope.is_empty() {
            return Err(
                "search for coherences and scopes, such as B.X or (K, [K] L), or for rules, such as [B, C] D, not both",
            );
        }
        Ok(Self::Rule(
            program.rule.iter().map(Definition::canonical).collect(),
        ))
    }
}

impl Region {
    // A pattern's own parts are found anywhere: a coherence in any scope, a scope at any depth.
    pub fn configuration(entry: &Configuration) -> Self {
        Self {
            coherence: (0..entry.coherence.len()).collect(),
            frame: (1..entry.frame.len()).collect(),
            rule: Vec::new(),
        }
    }

    pub fn frame(entry: &Configuration, frame: usize) -> Self {
        Self {
            coherence: (0..entry.coherence.len())
                .filter(|&index| entry.coherence[index].frame == frame)
                .collect(),
            frame: (0..entry.frame.len())
                .filter(|&index| entry.frame[index].parent == Some(frame))
                .collect(),
            rule: entry.frame[frame]
                .rule
                .iter()
                .filter_map(|occurrence| match occurrence.value {
                    Value::Rule(rule) => Some(rule),
                    Value::Atom(_) => None,
                })
                .collect(),
        }
    }

    // Only a scope the program opens at the start can equal a target's scope: a scope a rule
    // opens holds what the rule matched, which no text names.
    pub fn opened(entry: &Configuration, frame: usize) -> Self {
        let mut region = Self::frame(entry, frame);
        region
            .frame
            .retain(|&index| entry.frame[index].opener == Some(Opener::Program));
        region
    }
}

pub fn same(item: &Item, value: &Value, canon: &(impl Canon + ?Sized)) -> bool {
    match (item, value) {
        (Item::Atom(atom), Value::Atom(other)) => atom == other,
        (Item::Rule(definition), Value::Rule(rule)) => canon.canonical(*rule) == Some(definition),
        _ => false,
    }
}

pub fn cover(
    particle: &[Item],
    coherence: &Coherence,
    canon: &(impl Canon + ?Sized),
) -> Option<Vec<usize>> {
    let mut used = Vec::new();
    for item in particle {
        let found = coherence.occurrence.iter().find(|occurrence| {
            !used.contains(&occurrence.id) && same(item, &occurrence.value, canon)
        })?;
        used.push(found.id);
    }
    Some(used)
}

fn augment<Fit>(
    row: usize,
    fit: &[Vec<Option<Fit>>],
    taken: &[bool],
    owner: &mut [Option<usize>],
    seen: &mut [bool],
) -> bool {
    for column in 0..taken.len() {
        if taken[column] || seen[column] || fit[row][column].is_none() {
            continue;
        }
        seen[column] = true;
        if owner[column].is_none_or(|other| augment(other, fit, taken, owner, seen)) {
            owner[column] = Some(row);
            return true;
        }
    }
    false
}

fn completable<Fit>(fit: &[Vec<Option<Fit>>], taken: &[bool]) -> bool {
    let mut owner = vec![None; taken.len()];
    (0..fit.len()).all(|row| augment(row, fit, taken, &mut owner, &mut vec![false; taken.len()]))
}

// Each part takes the first place that leaves the remaining parts a perfect matching, which is the
// assignment a depth-first search finds, without its factorial cost.
fn choose<Fit: Clone>(fit: &[Vec<Option<Fit>>], width: usize) -> Option<Vec<Fit>> {
    let mut taken = vec![false; width];
    let mut found = Vec::new();
    for row in 0..fit.len() {
        let mut chosen = None;
        for column in 0..width {
            if taken[column] || fit[row][column].is_none() {
                continue;
            }
            taken[column] = true;
            if completable(&fit[row + 1..], &taken) {
                chosen = Some(column);
                break;
            }
            taken[column] = false;
        }
        found.push(fit[row][chosen?].clone()?);
    }
    Some(found)
}

// Parts listed together take different places, and a scope's parts are found inside the scope it
// takes, so the order parts are written in never matters.
pub fn embed(
    body: &Body,
    region: &Region,
    entry: &Configuration,
    canon: &(impl Canon + ?Sized),
) -> Option<Embedding> {
    let mut live = region.rule.clone();
    for definition in &body.rule {
        let position = live
            .iter()
            .position(|&rule| canon.canonical(rule) == Some(definition))?;
        live.swap_remove(position);
    }
    let coherence = body
        .coherence
        .iter()
        .map(|particle| {
            region
                .coherence
                .iter()
                .map(|&index| {
                    cover(particle, &entry.coherence[index], canon).map(|occurrence| Match {
                        coherence: index,
                        occurrence,
                    })
                })
                .collect()
        })
        .collect::<Vec<Vec<_>>>();
    let scope = body
        .scope
        .iter()
        .map(|scope| {
            region
                .frame
                .iter()
                .map(|&frame| {
                    let region = Region::frame(entry, frame);
                    embed(scope, &region, entry, canon).map(|mut inner| {
                        inner.frame.insert(0, frame);
                        inner
                    })
                })
                .collect()
        })
        .collect::<Vec<Vec<_>>>();
    let mut embedding = Embedding {
        coherence: choose(&coherence, region.coherence.len())?,
        frame: Vec::new(),
    };
    for inner in choose(&scope, region.frame.len())? {
        embedding.coherence.extend(inner.coherence);
        embedding.frame.extend(inner.frame);
    }
    Some(embedding)
}

pub fn assign(
    body: &Body,
    entry: &Configuration,
    canon: &(impl Canon + ?Sized),
) -> Option<Embedding> {
    embed(body, &Region::configuration(entry), entry, canon)
}

pub fn rule(pattern: &[Definition], canon: &(impl Canon + ?Sized), count: usize) -> Vec<usize> {
    (0..count)
        .filter(|&index| {
            canon
                .canonical(index)
                .is_some_and(|definition| pattern.contains(definition))
        })
        .collect()
}
