use crate::state::{Canonical, Frame};
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Default)]
pub(crate) struct Store {
    frame: BTreeSet<Arc<Frame>>,
}

impl Store {
    pub fn insert(&mut self, mut canonical: Canonical) -> Canonical {
        for index in 0..canonical.state.frame.len() {
            let value = &mut canonical.state.frame[index];
            if let Some(previous) = self.frame.get(value) {
                *value = previous.clone();
                continue;
            }
            self.frame.insert(value.clone());
        }
        canonical
    }
}

#[cfg(test)]
#[path = "../test/preservation.rs"]
mod test;
