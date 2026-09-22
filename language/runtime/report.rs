use super::Runtime;
use crate::render::{Builder, Sequence};
use crate::snapshot::{Event, Link, Node, Snapshot, View};
use crate::support::{Atom, Status};
use serde::Serialize;

impl Runtime {
    pub(crate) fn status(&self, index: usize) -> Status {
        self.proof.status(Atom::State(index))
    }

    fn node(&self) -> impl Iterator<Item = Node> + '_ {
        let support = self.proof.evaluate();
        let mut builder = Builder::new(&self.program);
        self.state.iter().enumerate().map(move |(index, state)| {
            builder.node(index, state, support.status(Atom::State(index)))
        })
    }

    fn transition(&self) -> impl Iterator<Item = Event> + '_ {
        let support = self.proof.evaluate();
        self.event
            .iter()
            .enumerate()
            .map(move |(index, event)| Event {
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
    }

    fn projection(&self) -> impl Iterator<Item = View> + '_ {
        let support = self.proof.evaluate();
        self.view.iter().enumerate().map(move |(index, view)| View {
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
    }

    fn assemble<State, Transition, Projection>(
        &self,
        state: State,
        event: Transition,
        view: Projection,
    ) -> Snapshot<State, Transition, Projection> {
        Snapshot {
            definition: Builder::new(&self.program).definition(),
            closed: self.closed(),
            record: self.record(),
            peak: self.peak,
            queued: self.agenda.len(),
            deferred: self.pending.len(),
            work: self.work,
            limit: self.limit,
            state,
            event,
            view,
        }
    }

    pub fn view(&self) -> impl Serialize + '_ {
        self.assemble(
            Sequence::new(|| self.node()),
            Sequence::new(|| self.transition()),
            Sequence::new(|| self.projection()),
        )
    }

    pub fn snapshot(&self) -> Snapshot {
        self.assemble(
            self.node().collect(),
            self.transition().collect(),
            self.projection().collect(),
        )
    }
}
