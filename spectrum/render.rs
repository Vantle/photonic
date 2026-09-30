use crate::configuration::{Coherence, Configuration, Occurrence, Value};
use crate::exploration::{self, Exploration, Rule};
use crate::handle::Handle;
use crate::inspect::{Item, Move};
use crate::recording::Mode;
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

// A scope's frame, named with the frames it sits in, innermost first, such as f2 in f1.
pub fn frame(configuration: &Configuration, index: usize) -> String {
    std::iter::successors(Some(index), |&current| {
        configuration.frame.get(current)?.parent
    })
    .take_while(|&current| current != 0)
    .map(|current| format!("f{current}"))
    .collect::<Vec<_>>()
    .join(" in ")
}

// A configuration's coherences, those of the root first and then each scope's after the frames it
// sits in, with the rules its rule values name; with live, each scope also lists the rules live in
// it. The root's rules are the program's, so they are never listed.
fn written(rule: &[Rule], configuration: &Configuration, live: bool) -> String {
    let inside = |frame: usize| {
        let scoped = configuration
            .frame
            .get(frame)
            .filter(|_| live && frame != 0)
            .map_or(&[][..], |entry| entry.rule.as_slice());
        configuration
            .coherence
            .iter()
            .filter(|entry| entry.frame == frame)
            .map(|entry| coherence(rule, entry))
            .chain(scoped.iter().map(|entry| match &entry.value {
                Value::Rule(index) => rule[*index].text.clone(),
                Value::Atom(atom) => atom.clone(),
            }))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let part = std::iter::once(inside(0))
        .filter(|root| !root.is_empty())
        .chain(
            (1..configuration.frame.len())
                .map(|index| (index, inside(index)))
                .filter(|(_, text)| !text.is_empty())
                .map(|(index, text)| format!("in {}: {text}", frame(configuration, index))),
        )
        .collect::<Vec<_>>();
    if part.is_empty() {
        return "nothing".to_owned();
    }
    part.join(" · ")
}

pub fn text(rule: &[Rule], configuration: &Configuration) -> String {
    written(rule, configuration, false)
}

pub fn configuration(exploration: &Exploration, index: usize) -> String {
    exploration
        .configuration
        .get(index)
        .map(|configuration| text(&exploration.rule, configuration))
        .unwrap_or_default()
}

// A configuration with the rules live in each scope, which compare tells configurations apart by.
pub fn detail(exploration: &Exploration, index: usize) -> String {
    exploration
        .configuration
        .get(index)
        .map(|configuration| written(&exploration.rule, configuration, true))
        .unwrap_or_default()
}

// Whether a configuration holds a coherence inside a scope.
pub fn scope(configuration: &Configuration) -> bool {
    configuration
        .coherence
        .iter()
        .any(|coherence| coherence.frame != 0)
}

// How far an exploration's answers reach: every future, or every plain schedule, once it closed;
// what it explored so far while it is open; or the one run a direct path follows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Extent {
    Closed,
    Open,
    Path,
}

impl Extent {
    pub fn new(mode: Mode, closed: bool) -> Self {
        match (mode, closed) {
            (Mode::Path, _) => Self::Path,
            (Mode::Exhaustive | Mode::Plain, true) => Self::Closed,
            (Mode::Exhaustive | Mode::Plain, false) => Self::Open,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Open => "open",
            Self::Path => "a direct path",
        }
    }

    // That a rule did not fire, in words that claim no more than the exploration settled.
    pub fn never(self) -> &'static str {
        match self {
            Self::Closed => "never",
            Self::Open => "not yet",
            Self::Path => "not on this path",
        }
    }

    // That the events listed at a configuration may not be all that can happen there.
    pub fn partial(self) -> Option<&'static str> {
        match self {
            Self::Closed => None,
            Self::Open => Some("the exploration is open; more events may happen here"),
            Self::Path => {
                Some("a direct path records only the event it took; more may happen here")
            }
        }
    }

    // How often a rule fired, in words that claim no more than the exploration settled.
    pub fn firing(self, fired: usize) -> String {
        let time = count(fired, "time");
        match (self, fired) {
            (Self::Closed, 0) => "never fires".to_owned(),
            (Self::Open, 0) => "has not fired yet".to_owned(),
            (Self::Path, 0) => "does not fire on this path".to_owned(),
            (Self::Closed, _) => format!("fires {time}"),
            (Self::Open, _) => format!("has fired {time} so far"),
            (Self::Path, _) => format!("fires {time} on this path"),
        }
    }
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
            exploration.rule[entry.rule].text
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
        name: String::new(),
        input: definition.input.clone(),
        output: Vec::new(),
    })
}
