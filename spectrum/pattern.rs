use crate::exploration::{Coherence, Exploration, Value};
use crate::failure::{Code, Failure};
use photonic::source::{self, Definition};
use photonic::syntax::Kind;

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

fn shorthand(text: &str) -> Result<(String, Vec<usize>), Failure> {
    let tree = photonic::parser::parse(text)
        .map_err(|error| Failure::located(Code::Pattern, &error, "pattern", text))?;
    let node = tree.node();
    let child = |parent: usize| {
        node.iter()
            .enumerate()
            .filter(move |(_, entry)| entry.parent == Some(parent))
            .map(|(index, _)| index)
    };
    let bracketed = |term: usize| child(term).any(|index| node[index].kind == Kind::Rule);
    let mut insertion = Vec::new();
    for list in child(0).filter(|&index| node[index].kind == Kind::List) {
        for term in child(list) {
            let factor = child(term).collect::<Vec<_>>();
            let [group] = factor.as_slice() else {
                continue;
            };
            if node[*group].kind != Kind::Group {
                continue;
            }
            let inner = child(*group)
                .filter(|&index| node[index].kind == Kind::List)
                .flat_map(child)
                .collect::<Vec<_>>();
            if !inner.is_empty() && inner.iter().all(|&term| bracketed(term)) {
                insertion.push(node[*group].span.start);
            }
        }
    }
    let mut result = text.to_owned();
    for &offset in insertion.iter().rev() {
        result.insert_str(offset, "().");
    }
    Ok((result, insertion))
}

fn original(offset: usize, insertion: &[usize]) -> usize {
    let mut shift = 0;
    for &position in insertion {
        if position + shift < offset {
            shift += 3;
        }
    }
    offset.saturating_sub(shift)
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
                "write a pattern, such as B.X, B, C, ([A] B) or [B, C] D",
            ));
        }
        let (rewritten, insertion) = shorthand(text)?;
        let program = photonic::lowering::parse(&rewritten).map_err(|error| {
            Failure::shifted(Code::Pattern, &error, "pattern", text, |offset| {
                original(offset, &insertion)
            })
        })?;
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

fn place(
    pattern: &[Vec<Item>],
    coherence: &[Coherence],
    exploration: &Exploration,
    found: &mut Vec<Match>,
) -> bool {
    let Some(particle) = pattern.get(found.len()) else {
        return true;
    };
    for (index, entry) in coherence.iter().enumerate() {
        if found.iter().any(|entry| entry.coherence == index) {
            continue;
        }
        let Some(occurrence) = cover(particle, entry, exploration) else {
            continue;
        };
        found.push(Match {
            coherence: index,
            occurrence,
        });
        if place(pattern, coherence, exploration, found) {
            return true;
        }
        found.pop();
    }
    false
}

pub fn assign(
    pattern: &[Vec<Item>],
    exploration: &Exploration,
    configuration: usize,
) -> Option<Vec<Match>> {
    let coherence = &exploration.configuration.get(configuration)?.coherence;
    let mut found = Vec::new();
    place(pattern, coherence, exploration, &mut found).then_some(found)
}

pub fn rule(pattern: &[Definition], exploration: &Exploration) -> Vec<usize> {
    (0..exploration.rule.len())
        .filter(|&index| pattern.contains(&exploration.rule[index].canonical))
        .collect()
}
