use super::space::Space;
use crate::term::Term;

#[derive(Eq, Hash, PartialEq)]
pub(super) struct Key {
    frame: usize,
    pattern: Vec<Vec<Term>>,
}

impl Key {
    pub fn new(space: &Space, order: &[usize]) -> Self {
        Self {
            frame: space.frame,
            pattern: order
                .iter()
                .map(|&position| space.pattern[position].clone())
                .collect(),
        }
    }

    pub fn retained(&self) -> usize {
        1 + self
            .pattern
            .iter()
            .map(|particle| particle.len() + 1)
            .sum::<usize>()
    }
}
