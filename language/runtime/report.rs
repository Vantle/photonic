use super::Runtime;
use crate::flow::Flow;
use crate::prism::{Reach, Verdict};
use crate::render::{Builder, Sequence};
use crate::snapshot::{Event, Link, Node, Snapshot, View};
use crate::status::Status;
use crate::support::Atom;
use serde::Serialize;

fn context(flow: &Flow) -> Vec<Vec<usize>> {
    flow.context
        .iter()
        .map(|value| value.iter().copied().collect())
        .collect()
}

fn link(flow: &Flow) -> Vec<Link> {
    flow.resource
        .iter()
        .map(|(&target, source)| Link {
            target,
            source: source.iter().copied().collect(),
        })
        .collect()
}

impl Runtime {
    pub(crate) fn status(&self, index: usize) -> Status {
        self.proof.status(Atom::State(index))
    }

    pub fn verdict(&self, target: &frontend::source::Program) -> Verdict {
        crate::prism::verdict(
            &self.program,
            &self.state,
            |index| self.status(index),
            self.closed(),
            target,
        )
    }

    pub fn reach(&self) -> Reach {
        let support = self.proof.evaluate();
        Reach::new(
            self.program.clone(),
            self.state.clone(),
            (0..self.state.len())
                .map(|index| support.status(Atom::State(index)))
                .collect(),
            self.closed(),
        )
    }

    pub fn resource(&self, event: usize) -> Option<Vec<Link>> {
        self.event.get(event).map(|event| link(&event.flow))
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
                rule: event.identity.rule,
                status: support.status(Atom::Event(index)),
                footprint: event.identity.binding.footprint.iter().copied().collect(),
                exact: event.identity.binding.exact.iter().copied().collect(),
                read: event.identity.binding.read.iter().copied().collect(),
                evidence: event.evidence.iter().copied().collect(),
                world: event.identity.binding.world.iter().copied().collect(),
                context: context(&event.flow),
            })
    }

    fn projection(&self) -> impl Iterator<Item = View> + '_ {
        let support = self.proof.evaluate();
        self.view.iter().enumerate().map(move |(index, view)| View {
            id: index,
            source: view.source,
            target: view.target,
            status: support.status(Atom::View(index)),
            origin: self.origin[index],
            resource: link(&view.flow),
            context: context(&view.flow),
            frame: view.flow.frame.clone(),
        })
    }

    fn assemble<Configuration, Transition, Projection>(
        &self,
        state: Configuration,
        event: Transition,
        view: Projection,
    ) -> Snapshot<Configuration, Transition, Projection> {
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

    pub fn stream(&self) -> impl Serialize + '_ {
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
