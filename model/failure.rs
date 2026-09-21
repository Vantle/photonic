use crate::history;
use crate::scope;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Failure {
    Capacity,
    Rule,
    Generator,
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
