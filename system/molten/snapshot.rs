use crate::flow::Place;
use crate::program::{Program, Symbol};
use crate::runtime::Limit;
use crate::state::State;
use crate::support::Status;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Snapshot {
    pub closed: bool,
    pub record: usize,
    pub peak: usize,
    pub queued: usize,
    pub deferred: usize,
    pub work: usize,
    pub limit: Limit,
    pub state: Vec<Node>,
    pub event: Vec<Event>,
    pub view: Vec<View>,
    pub query: Vec<Query>,
}
#[derive(Debug, Serialize)]
pub struct Node {
    pub id: usize,
    pub world: Vec<World>,
    pub frame: Vec<Frame>,
    pub status: Status,
}
#[derive(Debug, Serialize)]
pub struct Token {
    pub id: usize,
    pub label: String,
    pub display: String,
    pub value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture: Option<usize>,
}
#[derive(Debug, Serialize)]
pub struct World {
    pub frame: usize,
    pub particle: Vec<Token>,
}
#[derive(Debug, Serialize)]
pub struct Frame {
    pub scope: String,
    pub parent: Option<usize>,
    pub lexical: Option<usize>,
    pub held: Vec<Token>,
}
#[derive(Debug, Serialize)]
pub struct Event {
    pub id: usize,
    pub source: usize,
    pub target: usize,
    pub rule: String,
    pub status: Status,
    pub footprint: Vec<Place>,
    pub exact: Vec<Place>,
    pub read: Vec<Place>,
    pub evidence: Vec<usize>,
}
#[derive(Debug, Serialize)]
pub struct Query {
    pub id: usize,
    pub source: usize,
    pub frame: usize,
    pub status: Status,
}

#[derive(Debug, Serialize)]
pub struct View {
    pub id: usize,
    pub source: usize,
    pub target: usize,
    pub status: Status,
    pub resource: Vec<Link>,
    pub context: Vec<Vec<usize>>,
    pub frame: Vec<Option<usize>>,
}

#[derive(Debug, Serialize)]
pub struct Link {
    pub target: Place,
    pub source: Vec<Place>,
}

impl Node {
    pub fn new(id: usize, state: &State, program: &Program, status: Status) -> Self {
        let particle = |value: &[crate::state::Token]| {
            value
                .iter()
                .map(|token| Token {
                    id: token.id,
                    label: match &token.value {
                        Symbol::Atom(index) => program.atom[*index].clone(),
                        Symbol::Rule(rule, _) => program.code.get(rule).map_or_else(
                            || program.label(&token.value),
                            |index| format!("§{index}"),
                        ),
                        _ => program.label(&token.value),
                    },
                    display: program.label(&token.value),
                    value: Value::new(&token.value, program),
                    capture: if let Symbol::Rule(_, capture) = &token.value {
                        *capture
                    } else {
                        None
                    },
                })
                .collect()
        };
        Self {
            id,
            world: state
                .world
                .iter()
                .map(|world| World {
                    frame: world.frame,
                    particle: particle(&world.particle),
                })
                .collect(),
            frame: state
                .frame
                .iter()
                .map(|frame| Frame {
                    scope: frame.scope.name.clone(),
                    parent: frame.parent,
                    lexical: frame.lexical,
                    held: particle(&frame.held),
                })
                .collect(),
            status,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Value {
    Atom {
        name: String,
    },
    Variable {
        name: String,
    },
    Structure {
        name: String,
        particle: Vec<Value>,
    },
    Rule {
        input: Vec<Vec<Value>>,
        output: Vec<Output>,
        negative: Option<Vec<Vec<Value>>>,
        capture: Option<usize>,
    },
}

#[derive(Debug, Serialize)]
pub struct Output {
    pub particle: Vec<Value>,
    pub body: Option<Vec<Value>>,
}

impl Value {
    fn new(value: &Symbol, program: &Program) -> Self {
        let particle = |value: &[Symbol]| {
            value
                .iter()
                .map(|value| Self::new(value, program))
                .collect()
        };
        match value {
            Symbol::Atom(index) => Self::Atom {
                name: program.atom[*index].clone(),
            },
            Symbol::Variable(name) => Self::Variable { name: name.clone() },
            Symbol::Structure(index, content) => Self::Structure {
                name: program.atom[*index].clone(),
                particle: particle(content),
            },
            Symbol::Rule(rule, capture) => Self::Rule {
                input: rule.input.iter().map(|value| particle(value)).collect(),
                output: rule
                    .output
                    .iter()
                    .map(|output| Output {
                        particle: particle(&output.particle),
                        body: output.body.as_ref().map(|scope| {
                            scope
                                .rule
                                .iter()
                                .map(|rule| Self::new(&Symbol::Rule(rule.clone(), None), program))
                                .collect()
                        }),
                    })
                    .collect(),
                negative: rule
                    .negative
                    .as_ref()
                    .map(|value| value.iter().map(|value| particle(value)).collect()),
                capture: *capture,
            },
        }
    }
}
