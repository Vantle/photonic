use crate::exploration::{Coherence, Exploration, Value};
use crate::failure::{Code, Failure};
use frontend::source::{self, Definition};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Item {
    Atom(String),
    Rule(Definition),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Pattern {
    Coherence(Vec<Vec<Item>>),
    Rule(Vec<Definition>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Match {
    pub coherence: usize,
    pub occurrence: Vec<usize>,
}

pub fn item(value: &source::Value) -> Item {
    match value {
        source::Value::Atom(atom) => Item::Atom(atom.clone()),
        source::Value::Rule { rule } => Item::Rule(rule.canonical()),
    }
}

impl Pattern {
    pub fn read(text: &str) -> Result<Self, Failure> {
        if text.trim().is_empty() {
            return Err(Failure::new(
                Code::Pattern,
                "write a pattern, such as B.X, B, C, ().([A] B) or [B, C] D",
            ));
        }
        let program = frontend::lowering::parse(text)
            .map_err(|error| Failure::located(Code::Pattern, &error, "pattern", text))?;
        if !program.scope.is_empty() {
            return Err(Failure::new(
                Code::Pattern,
                "a pattern matches coherences or rules, and a group that lists a rule is a scope; to match a rule inside a coherence, join it, as in ().([A] B)",
            ));
        }
        if !program.rule.is_empty() && !program.initial.is_empty() {
            return Err(Failure::new(
                Code::Pattern,
                "search for coherences or for rules, such as B.X or [B, C] D, not both",
            ));
        }
        if !program.rule.is_empty() {
            return Ok(Self::Rule(
                program.rule.iter().map(Definition::canonical).collect(),
            ));
        }
        Ok(Self::Coherence(
            program
                .initial
                .iter()
                .map(|particle| particle.iter().map(item).collect())
                .collect(),
        ))
    }
}

pub fn same(item: &Item, value: &Value, exploration: &Exploration) -> bool {
    match (item, value) {
        (Item::Atom(atom), Value::Atom(other)) => atom == other,
        (Item::Rule(definition), Value::Rule(rule)) => {
            exploration.rule[*rule].canonical == *definition
        }
        _ => false,
    }
}

pub fn cover(
    particle: &[Item],
    coherence: &Coherence,
    exploration: &Exploration,
) -> Option<Vec<usize>> {
    let mut used = Vec::new();
    for item in particle {
        let found = coherence.occurrence.iter().find(|occurrence| {
            !used.contains(&occurrence.id) && same(item, &occurrence.value, exploration)
        })?;
        used.push(found.id);
    }
    Some(used)
}

fn augment(
    row: usize,
    fit: &[Vec<Option<Vec<usize>>>],
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

fn completable(fit: &[Vec<Option<Vec<usize>>>], taken: &[bool]) -> bool {
    let mut owner = vec![None; taken.len()];
    (0..fit.len()).all(|row| augment(row, fit, taken, &mut owner, &mut vec![false; taken.len()]))
}

// Each part of the pattern takes the first coherence that leaves the remaining parts a perfect
// matching, which is the assignment a depth-first search finds, without its factorial cost.
pub fn assign(
    pattern: &[Vec<Item>],
    exploration: &Exploration,
    configuration: usize,
) -> Option<Vec<Match>> {
    let coherence = &exploration.configuration.get(configuration)?.coherence;
    let fit = pattern
        .iter()
        .map(|particle| {
            coherence
                .iter()
                .map(|entry| cover(particle, entry, exploration))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut taken = vec![false; coherence.len()];
    let mut found = Vec::new();
    for row in 0..fit.len() {
        let mut chosen = None;
        for column in 0..coherence.len() {
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
        let column = chosen?;
        found.push(Match {
            coherence: column,
            occurrence: fit[row][column].clone()?,
        });
    }
    Some(found)
}

pub fn rule(pattern: &[Definition], exploration: &Exploration) -> Vec<usize> {
    (0..exploration.rule.len())
        .filter(|&index| pattern.contains(&exploration.rule[index].canonical))
        .collect()
}
