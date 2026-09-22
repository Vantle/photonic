use crate::basis::Set;
use crate::flow::{Binding, Place};
use crate::state::Frame;
use std::sync::Arc;

pub(crate) struct Selection {
    occurrence: Set<Place>,
}

impl Selection {
    pub fn new(binding: &Binding) -> Self {
        Self {
            occurrence: binding
                .exact
                .iter()
                .copied()
                .filter(|place| matches!(place, Place::Context(_, _)))
                .collect(),
        }
    }

    pub fn frame(&self, index: usize, frame: &Frame) -> Option<Arc<Frame>> {
        if self.occurrence.len() == 0
            || !frame
                .particle
                .iter()
                .any(|token| self.occurrence.contains(&Place::Context(index, token.id)))
        {
            return None;
        }
        Some(Arc::new(Frame {
            particle: frame
                .particle
                .iter()
                .filter(|token| !self.occurrence.contains(&Place::Context(index, token.id)))
                .cloned()
                .collect(),
            ..frame.clone()
        }))
    }
}
