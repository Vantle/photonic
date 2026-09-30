use crate::configuration::{Coherence, Configuration, Occurrence, Value};
use crate::exploration::{self, Exploration, Rule};
use crate::handle::Handle;
use crate::inspect::{Item, Move};
use frontend::source;
use photonic::place::Place;
use serde::Serialize;

pub fn value(rule: &[Rule], value: &Value) -> source::Value {
    match value {
        Value::Atom(atom) => source::Value::Atom(atom.clone()),
        Value::Rule(index) => source::Value::Rule {
            rule: Box::new(rule[*index].definition.clone()),
        },
    }
}

pub fn particle(rule: &[Rule], occurrence: &[Occurrence]) -> String {
    let particle = occurrence
        .iter()
        .map(|entry| value(rule, &entry.value))
        .collect::<Vec<_>>();
    frontend::text::coherence(&particle)
}

pub fn occurrence(rule: &[Rule], occurrence: &Occurrence) -> String {
    match &occurrence.value {
        Value::Atom(atom) => atom.clone(),
        Value::Rule(index) => format!("({})", rule[*index].text),
    }
}

pub fn coherence(rule: &[Rule], coherence: &Coherence) -> String {
    particle(rule, &coherence.occurrence)
}

// A configuration's coherences, those of the root first and then each scope's, with the rules its
// rule values name.
pub fn text(rule: &[Rule], configuration: &Configuration) -> String {
    let inside = |frame: usize| {
        configuration
            .coherence
            .iter()
            .filter(|entry| entry.frame == frame)
            .map(|entry| coherence(rule, entry))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let root = inside(0);
    let part = std::iter::once(root)
        .filter(|root| !root.is_empty())
        .chain(
            (1..configuration.frame.len())
                .map(|frame| (frame, inside(frame)))
                .filter(|(_, text)| !text.is_empty())
                .map(|(frame, text)| format!("in f{frame}: {text}")),
        )
        .collect::<Vec<_>>();
    if part.is_empty() {
        return "nothing".to_owned();
    }
    part.join(" · ")
}

pub fn configuration(exploration: &Exploration, index: usize) -> String {
    exploration
        .configuration
        .get(index)
        .map(|configuration| text(&exploration.rule, configuration))
        .unwrap_or_default()
}

// Whether a configuration holds a coherence inside a scope.
pub fn scope(configuration: &Configuration) -> bool {
    configuration
        .coherence
        .iter()
        .any(|coherence| coherence.frame != 0)
}

pub fn name(value: impl Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_default()
}

pub fn count(value: usize, noun: &str) -> String {
    if value == 1 {
        return format!("1 {noun}");
    }
    format!("{value} {noun}s")
}

pub fn item(exploration: &Exploration, configuration: usize, id: usize) -> Item {
    Item {
        handle: Handle::Occurrence(configuration, id).to_string(),
        text: exploration
            .find(configuration, id)
            .map(|entry| occurrence(&exploration.rule, entry))
            .unwrap_or_default(),
    }
}

pub fn place(exploration: &Exploration, configuration: usize, place: Place) -> Item {
    item(exploration, configuration, exploration::id(place))
}

pub fn movement(exploration: &Exploration, event: usize, forward: bool) -> Move {
    let entry = &exploration.event[event];
    let configuration = if forward { entry.target } else { entry.source };
    Move {
        event: Handle::Event(event).to_string(),
        rule: format!(
            "{} {}",
            Handle::Rule(entry.rule),
            brief(exploration, entry.rule)
        ),
        configuration: Handle::Configuration(configuration).to_string(),
        text: self::configuration(exploration, configuration),
        inferred: exploration.inferred(event),
        unsupported: !entry.supported,
    }
}

pub fn table(label: &str, entry: &[Move], line: &mut Vec<String>) {
    let rule = entry
        .iter()
        .map(|entry| entry.rule.chars().count())
        .max()
        .unwrap_or(0);
    let configuration = entry
        .iter()
        .map(|entry| entry.configuration.len())
        .max()
        .unwrap_or(0);
    for (position, entry) in entry.iter().enumerate() {
        let note = match (entry.inferred, entry.unsupported) {
            (true, true) => "   inferred, unsupported",
            (true, false) => "   inferred",
            (false, true) => "   unsupported",
            (false, false) => "",
        };
        line.push(format!(
            "{}{:<5} {:<rule$}   {:<configuration$} {}{note}",
            row(label, position == 0),
            entry.event,
            entry.rule,
            entry.configuration,
            entry.text
        ));
    }
}

pub fn row(label: &str, first: bool) -> String {
    let label = if first { label } else { "" };
    format!("{label:<11}")
}

pub fn input(exploration: &Exploration, rule: usize) -> String {
    let definition = &exploration.rule[rule].definition;
    frontend::text::definition(&source::Definition {
        input: definition.input.clone(),
        output: Vec::new(),
    })
}

pub fn brief(exploration: &Exploration, rule: usize) -> String {
    let entry = &exploration.rule[rule];
    if entry
        .definition
        .output
        .iter()
        .all(|output| matches!(output, source::Output::Particle(_)))
    {
        return entry.text.clone();
    }
    format!("{} (…)", input(exploration, rule))
}
