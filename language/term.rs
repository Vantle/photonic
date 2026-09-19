use crate::program::Symbol;
use crate::state::Token;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct Term {
    pub value: Symbol,
    pub capture: Option<usize>,
}

impl Term {
    pub fn new(value: Symbol, capture: Option<usize>) -> Self {
        Self {
            value,
            capture: capture.filter(|_| matches!(value, Symbol::Rule(_))),
        }
    }

    pub fn matches(&self, token: &Token) -> bool {
        token.value == self.value
            && (matches!(self.value, Symbol::Atom(_)) || token.capture == self.capture)
    }
}
