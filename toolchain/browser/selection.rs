use crate::failure::{Code, Failure};
use crate::request;
use frontend::source::Definition;
use serde::{Deserialize, Serialize};
use spectrum::configuration::{Coherence, Configuration, Frame, Occurrence, Value};
use spectrum::pattern::{self, Body, Item, Pattern};

// The book sends back an execution the engine produced, so a selection may be as large as the
// explorations the engine's budgets allow.
const SIZE: usize = 67_108_864;

#[derive(Deserialize)]
struct Query {
    pattern: String,
    execution: Execution,
}

#[derive(Deserialize)]
struct Execution {
    definition: Vec<String>,
    state: Vec<State>,
    event: Vec<Event>,
}

#[derive(Deserialize)]
struct State {
    id: usize,
    world: Vec<World>,
    frame: Vec<Scope>,
}

#[derive(Deserialize)]
struct World {
    frame: usize,
    particle: Vec<Token>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum Token {
    Atom { id: usize, label: String },
    Rule { id: usize, rule: usize },
}

#[derive(Deserialize)]
struct Scope {
    parent: Option<usize>,
    particle: Vec<Reference>,
}

#[derive(Deserialize)]
struct Reference {
    id: usize,
    rule: usize,
}

#[derive(Deserialize)]
struct Event {
    id: usize,
    rule: String,
}

#[derive(Serialize)]
pub struct Covered {
    world: usize,
    token: Vec<usize>,
}

#[derive(Serialize)]
pub struct Found {
    id: usize,
    world: Vec<Covered>,
}

#[derive(Serialize)]
pub struct Lane {
    state: usize,
    world: usize,
    token: Vec<usize>,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Selection {
    Configuration { state: Vec<Found>, lane: Vec<Lane> },
    Event { event: Vec<usize> },
}

// The book names each rule by its text, so reading the text back gives the rule a pattern compares.
fn canonical(name: &str) -> Option<Definition> {
    let program = frontend::lowering::parse(name).ok()?;
    let [rule] = program.rule.as_slice() else {
        return None;
    };
    (program.initial.is_empty() && program.scope.is_empty()).then(|| rule.canonical())
}

fn occurrence(token: &Token) -> Occurrence {
    match token {
        Token::Atom { id, label } => Occurrence {
            id: *id,
            value: Value::Atom(label.clone()),
        },
        Token::Rule { id, rule } => Occurrence {
            id: *id,
            value: Value::Rule(*rule),
        },
    }
}

fn configuration(state: &State) -> Configuration {
    Configuration {
        coherence: state
            .world
            .iter()
            .map(|world| Coherence {
                frame: world.frame,
                occurrence: world.particle.iter().map(occurrence).collect(),
            })
            .collect(),
        frame: state
            .frame
            .iter()
            .map(|scope| Frame {
                opener: None,
                parent: scope.parent,
                lexical: None,
                rule: scope
                    .particle
                    .iter()
                    .map(|reference| Occurrence {
                        id: reference.id,
                        value: Value::Rule(reference.rule),
                    })
                    .collect(),
                held: Vec::new(),
            })
            .collect(),
        supported: true,
    }
}

fn coherence(body: &Body) -> Vec<&[Item]> {
    body.coherence
        .iter()
        .map(Vec::as_slice)
        .chain(body.scope.iter().flat_map(coherence))
        .collect()
}

pub fn select(input: &str) -> Result<Selection, Failure> {
    if input.len() > SIZE {
        return Err(Failure::new(Code::Size, "Keep the selection below 64 MiB."));
    }
    let query: Query = request::decode(input)?;
    let program = frontend::lowering::parse(&query.pattern)
        .map_err(|error| Failure::located(Code::Pattern, &error, &query.pattern))?;
    let pattern = Pattern::new(&program).map_err(|message| Failure::new(Code::Pattern, message))?;
    let canon = query
        .execution
        .definition
        .iter()
        .map(|name| canonical(name))
        .collect::<Vec<_>>();
    let body = match pattern {
        Pattern::Rule(rule) => {
            let name = pattern::rule(&rule, canon.as_slice(), canon.len())
                .into_iter()
                .map(|index| query.execution.definition[index].as_str())
                .collect::<Vec<_>>();
            return Ok(Selection::Event {
                event: query
                    .execution
                    .event
                    .iter()
                    .filter(|event| name.contains(&event.rule.as_str()))
                    .map(|event| event.id)
                    .collect(),
            });
        }
        Pattern::Configuration(body) => body,
    };
    let particle = coherence(&body);
    let mut state = Vec::new();
    let mut lane = Vec::new();
    for entry in &query.execution.state {
        let view = configuration(entry);
        if let Some(found) = pattern::assign(&body, &view, canon.as_slice()) {
            state.push(Found {
                id: entry.id,
                world: found
                    .coherence
                    .into_iter()
                    .map(|matched| Covered {
                        world: matched.coherence,
                        token: matched.occurrence,
                    })
                    .collect(),
            });
        }
        for (index, world) in view.coherence.iter().enumerate() {
            if let Some(token) = particle
                .iter()
                .find_map(|particle| pattern::cover(particle, world, canon.as_slice()))
            {
                lane.push(Lane {
                    state: entry.id,
                    world: index,
                    token,
                });
            }
        }
    }
    Ok(Selection::Configuration { state, lane })
}
