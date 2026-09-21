use crate::history;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Failure {
    Rule,
    History {
        identity: history::Identity,
        expected: history::Branch,
        actual: Option<history::Branch>,
    },
}
