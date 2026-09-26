use crate::place::Place;
use crate::runtime::Limit;
use crate::status::Status;
use frontend::source;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct Snapshot<Configuration = Vec<Node>, Transition = Vec<Event>, Projection = Vec<View>> {
    pub definition: Vec<Definition>,
    pub closed: bool,
    pub record: usize,
    pub peak: usize,
    pub queued: usize,
    pub deferred: usize,
    pub work: usize,
    pub limit: Limit,
    pub state: Configuration,
    pub event: Transition,
    pub view: Projection,
}

#[derive(Debug, Serialize)]
pub struct Definition {
    pub name: String,
    pub rule: source::Definition,
}

#[derive(Debug, Serialize)]
pub struct Node {
    pub id: usize,
    pub world: Vec<World>,
    pub frame: Vec<Frame>,
    pub status: Status,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Value {
    Atom(Arc<str>),
    Rule(usize),
}

#[derive(Debug, Serialize)]
pub struct Token {
    pub id: usize,
    #[serde(flatten)]
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
    pub scope: Arc<str>,
    pub opener: Option<usize>,
    pub parent: Option<usize>,
    pub lexical: Option<usize>,
    pub particle: Vec<Token>,
    pub held: Vec<Token>,
}

#[derive(Debug, Serialize)]
pub struct Event {
    pub id: usize,
    pub source: usize,
    pub target: usize,
    pub rule: usize,
    pub status: Status,
    pub footprint: Vec<Place>,
    pub exact: Vec<Place>,
    pub read: Vec<Place>,
    pub evidence: Vec<usize>,
    pub world: Vec<usize>,
    pub context: Vec<Vec<usize>>,
}

#[derive(Debug, Serialize)]
pub struct View {
    pub id: usize,
    pub source: usize,
    pub target: usize,
    pub status: Status,
    pub origin: Option<Origin>,
    pub resource: Vec<Link>,
    pub context: Vec<Vec<usize>>,
    pub frame: Vec<Option<usize>>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Origin {
    pub view: usize,
    pub event: usize,
}

#[derive(Debug, Serialize)]
pub struct Link {
    pub target: Place,
    pub source: Vec<Place>,
}

impl Snapshot {
    pub fn deduction(&self, event: usize) -> Vec<usize> {
        let evidence = &self.event[event].evidence;
        if evidence
            .iter()
            .any(|&index| self.view[index].source == self.view[index].target)
        {
            return Vec::new();
        }
        let Some(&first) = evidence.first() else {
            return Vec::new();
        };
        let mut chain = Vec::new();
        let mut cursor = self.view[first].origin;
        while let Some(origin) = cursor {
            chain.push(origin.event);
            cursor = self.view[origin.view].origin;
        }
        chain.reverse();
        chain
    }
}
