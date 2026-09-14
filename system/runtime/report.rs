use super::Runtime;
use crate::snapshot::{Event, Link, Node, Snapshot, View};
use crate::support::{Atom, Status, Support};

impl Runtime {
    pub(crate) fn status(&self, index: usize) -> Status {
        self.evaluation
            .get_or_init(|| Support::new(self.clause.iter().cloned()))
            .status(Atom::State(index))
    }

    pub fn snapshot(&self) -> Snapshot {
        let support = self
            .evaluation
            .get_or_init(|| Support::new(self.clause.iter().cloned()));
        Snapshot {
            closed: self.closed(),
            record: self.record(),
            peak: self.peak,
            queued: self.agenda.len(),
            deferred: self.pending.len(),
            work: self.work,
            limit: self.limit,
            state: self
                .state
                .iter()
                .enumerate()
                .map(|(index, state)| {
                    Node::new(
                        index,
                        state,
                        &self.program,
                        support.status(Atom::State(index)),
                    )
                })
                .collect(),
            event: self
                .event
                .iter()
                .enumerate()
                .map(|(index, event)| Event {
                    id: index,
                    source: event.identity.source,
                    target: event.target,
                    rule: self.program.rule[event.identity.rule].name.clone(),
                    status: support.status(Atom::Event(index)),
                    footprint: event.identity.binding.footprint.iter().copied().collect(),
                    exact: event.identity.binding.exact.iter().copied().collect(),
                    read: event.identity.binding.read.iter().copied().collect(),
                    evidence: event.evidence.iter().copied().collect(),
                })
                .collect(),
            view: self
                .view
                .iter()
                .enumerate()
                .map(|(index, view)| View {
                    id: index,
                    source: view.source,
                    target: view.target,
                    status: support.status(Atom::View(index)),
                    resource: view
                        .flow
                        .resource
                        .iter()
                        .map(|(&target, source)| Link {
                            target,
                            source: source.iter().copied().collect(),
                        })
                        .collect(),
                    context: view
                        .flow
                        .context
                        .iter()
                        .map(|value| value.iter().copied().collect())
                        .collect(),
                    frame: view.flow.frame.clone(),
                })
                .collect(),
        }
    }
}
