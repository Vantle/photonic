use crate::history;
use crate::scope;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Failure {
    Capacity,
    Depth(usize),
    Flow(crate::flow::Failure),
    Origin(crate::context::Identity),
    Held(crate::flow::Place),
    Rule,
    Generator,
    Site,
    Source,
    Owner {
        world: crate::world::Identity,
        context: crate::context::Identity,
    },
    State(usize),
    Derivation {
        depth: usize,
        position: usize,
    },
    Witness(crate::occurrence::Identity),
    Lineage {
        source: crate::world::Identity,
        target: crate::world::Identity,
    },
    Context(crate::context::Identity),
    World(crate::world::Identity),
    Match(crate::world::Identity),
    Occurrence(crate::occurrence::Identity),
    Repeated(crate::occurrence::Identity),
    Identity(crate::occurrence::Identity),
    Declaration {
        context: crate::context::Identity,
        position: usize,
    },
    Arity {
        expected: usize,
        actual: usize,
    },
    Sort {
        expected: crate::parameter::Sort,
        actual: crate::parameter::Sort,
    },
    Scope(scope::Identity),
    Binding {
        scope: scope::Identity,
        position: usize,
    },
    Occupied {
        scope: scope::Identity,
        position: usize,
    },
    History {
        identity: history::Identity,
        expected: history::Branch,
        actual: Option<history::Branch>,
    },
}
