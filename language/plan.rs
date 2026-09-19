use crate::program::Symbol;
use crate::term::Term;
use std::sync::Arc;

pub(crate) struct Input {
    value: Vec<Vec<Symbol>>,
    pattern: Arc<Vec<Vec<Term>>>,
    capture: bool,
}

impl Input {
    pub fn new(value: &[Vec<Symbol>]) -> Self {
        let pattern = Arc::new(if value.is_empty() {
            vec![Vec::new()]
        } else {
            pattern(value, None)
        });
        let capture = value
            .iter()
            .flatten()
            .any(|symbol| matches!(symbol, Symbol::Rule(_)));
        Self {
            value: value.to_vec(),
            pattern,
            capture,
        }
    }

    pub fn owner(&self, owner: usize) -> usize {
        if self.capture { owner } else { 0 }
    }

    pub fn pattern(&self, owner: usize) -> Arc<Vec<Vec<Term>>> {
        if self.capture {
            return Arc::new(pattern(&self.value, Some(owner)));
        }
        self.pattern.clone()
    }

    pub fn empty(&self) -> bool {
        self.value.is_empty() || self.value.iter().any(Vec::is_empty)
    }

    pub fn symbol(&self) -> impl Iterator<Item = Symbol> + '_ {
        self.value.iter().flatten().copied()
    }

    pub fn retained(&self) -> usize {
        self.value.len() * 2 + self.value.iter().map(Vec::len).sum::<usize>() * 2
    }
}

pub(crate) fn pattern(input: &[Vec<Symbol>], capture: Option<usize>) -> Vec<Vec<Term>> {
    input
        .iter()
        .map(|particle| {
            particle
                .iter()
                .map(|&value| Term::new(value, capture))
                .collect()
        })
        .collect()
}
