use crate::flow::Place;
use crate::runtime::Limit;
use crate::support::Status;
use serde::Serialize;
use std::sync::Arc;

#[derive(Debug, Serialize)]
pub struct Snapshot<State = Vec<Node>, Transition = Vec<Event>, Projection = Vec<View>> {
    pub definition: Vec<Definition>,
    pub closed: bool,
    pub record: usize,
    pub peak: usize,
    pub queued: usize,
    pub deferred: usize,
    pub work: usize,
    pub limit: Limit,
    pub state: State,
    pub event: Transition,
    pub view: Projection,
}
#[derive(Debug, Serialize)]
pub struct Definition {
    pub label: String,
    pub display: String,
}

#[derive(Serialize)]
struct Occurrence<'source> {
    id: usize,
    label: &'source str,
    #[serde(skip_serializing_if = "Option::is_none")]
    capture: Option<usize>,
}

fn particle<S: serde::Serializer>(value: &[Token], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(value.iter().map(|token| Occurrence {
        id: token.id,
        label: &token.label,
        capture: token.capture,
    }))
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
    pub label: Arc<str>,
    pub display: Arc<str>,
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
    pub parent: Option<usize>,
    pub lexical: Option<usize>,
    #[serde(serialize_with = "particle")]
    pub particle: Vec<Token>,
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
