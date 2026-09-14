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
    pub(crate) fn new(id: usize, state: &State, program: &Program, status: Status) -> Self {
        let particle = |value: &[crate::state::Token]| {
            value
                .iter()
                .map(|token| Token {
                    id: token.id,
                    label: match token.value {
                        Symbol::Atom(index) => program.atom[index].clone(),
                        Symbol::Rule(index) => format!("§{}", program.code[&index]),
                    },
                    display: program.label(token.value),
                    capture: token.capture,
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
                    scope: program.scope[frame.scope].name.clone(),
                    parent: frame.parent,
                    lexical: frame.lexical,
                    held: particle(&frame.held),
                })
                .collect(),
            status,
        }
    }
}
