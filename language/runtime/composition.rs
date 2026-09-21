use super::{Runtime, View};
use crate::support::Atom;
use std::sync::Arc;

impl Runtime {
    pub(super) fn compose(&mut self, previous: usize, event: usize) {
        let parent = &self.view[previous];
        let flow = if self.incoming[parent.target][0] == previous {
            self.event[event].flow.clone()
        } else {
            Arc::new(
                self.composition
                    .compose(&parent.flow, &self.event[event].flow),
            )
        };
        let view = self.witness(View {
            source: parent.source,
            target: self.event[event].target,
            flow,
        });
        self.support(Atom::View(view), [Atom::View(previous), Atom::Event(event)]);
    }
}

#[cfg(test)]
#[path = "../test/composition.rs"]
mod test;
