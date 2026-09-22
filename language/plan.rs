use crate::program::Symbol;
use crate::term::Term;
use smallvec::SmallVec;
use std::sync::Arc;

mod context;

pub(crate) use context::Context;

struct Shape {
    fragment: Vec<Arc<crate::pattern::Pattern>>,
    group: Vec<usize>,
    pattern: Vec<Vec<Symbol>>,
}

pub(crate) struct Input {
    shape: Arc<Shape>,
    capture: bool,
    arity: usize,
    empty: bool,
    dependency: SmallVec<[Symbol; 2]>,
}

impl Input {
    #[cfg(test)]
    pub fn new(value: &[Vec<Symbol>]) -> Self {
        Self::shared(value, &mut Default::default())
    }

    pub fn shared(
        value: &[Vec<Symbol>],
        shared: &mut std::collections::HashMap<
            Vec<Symbol>,
            Arc<crate::pattern::Pattern>,
            crate::hashing::Builder,
        >,
    ) -> Self {
        let pattern = value.to_vec();
        let mut dependency = value
            .iter()
            .flatten()
            .copied()
            .collect::<SmallVec<[Symbol; 2]>>();
        dependency.sort_unstable();
        dependency.dedup();
        let capture = dependency
            .iter()
            .any(|symbol| matches!(symbol, Symbol::Rule(_)));
        Self {
            shape: Arc::new(Shape {
                group: crate::partition::classify(&pattern),
                fragment: pattern
                    .iter()
                    .map(|value| {
                        shared
                            .entry(value.clone())
                            .or_insert_with(|| Arc::new(crate::pattern::Pattern::new(value)))
                            .clone()
                    })
                    .collect(),
                pattern,
            }),
            capture,
            arity: value.len(),
            empty: value.is_empty() || value.iter().any(Vec::is_empty),
            dependency,
        }
    }

    pub fn arity(&self) -> usize {
        self.arity
    }

    pub fn owner(&self, owner: usize) -> usize {
        if self.capture { owner } else { 0 }
    }

    #[cfg(test)]
    pub fn pattern(&self, owner: usize) -> Arc<Vec<Vec<Term>>> {
        Arc::new(pattern(&self.shape.pattern, Some(owner)))
    }

    pub fn context(&self, owner: usize) -> Context {
        Context::new(self.shape.clone(), owner)
    }

    pub fn empty(&self) -> bool {
        self.empty
    }

    pub fn dependency(&self) -> &[Symbol] {
        &self.dependency
    }

    pub fn retained(&self) -> usize {
        self.arity * 2
            + self.shape.pattern.iter().map(Vec::len).sum::<usize>() * 2
            + self.dependency.len()
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

#[cfg(test)]
#[path = "test/plan.rs"]
mod test;
