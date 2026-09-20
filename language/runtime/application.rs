use super::normalization::Status;
use super::{Application, Event, Identity, Runtime, Task};
use crate::flow::Closure;
use crate::support::Atom;
use std::collections::BTreeSet;
use std::sync::Arc;

impl Runtime {
    pub(super) fn apply(&mut self, application: Application) {
        let view = self.view[application.view].clone();
        let environment = application.capture.map(|capture| {
            self.normalization.environment(super::environment::Request {
                target: view.target,
                capture,
                state: &self.state[view.target],
            })
        });
        let key = Identity {
            source: view.source,
            frame: application.frame,
            owner: application.owner,
            rule: application.rule,
            binding: application.binding.clone(),
            environment,
        };
        match self.normalization.find(&key) {
            Some(Status::Complete(event)) => {
                self.justify(event, application);
                return;
            }
            Some(Status::Pending(index)) => {
                self.normalization.attach(index, application);
                return;
            }
            None => {}
        }
        let closure = application.capture.map(|capture| Closure {
            state: &self.state[view.target],
            flow: &view.flow,
            capture,
        });
        let result = crate::application::apply(crate::application::Request {
            source: &self.state[view.source],
            frame: application.frame,
            owner: application.owner,
            rule: &self.program.rule[application.rule],
            binding: &application.binding,
            closure,
        });
        if result.state.world.len() > self.limit.world
            || result.state.size() > self.limit.cell
            || result.state.reachable().len() > self.limit.frame
        {
            self.pending.insert(application);
            return;
        }
        let index = self.normalization.insert(key, application, result);
        self.agenda.push_back(Task::Normalize(index));
    }

    pub(super) fn normalize(
        &mut self,
        index: usize,
        search: crate::canonical::Search,
        complete: bool,
    ) {
        let Some(normalization) = self.normalization.advance(index, search, complete) else {
            self.agenda.push_back(Task::Normalize(index));
            return;
        };
        let result = normalization.result;
        if self.state.len() >= self.limit.state && !self.state.contains(&result.state) {
            self.pending.extend(normalization.application);
            return;
        }
        let source = normalization.identity.source;
        let target = self.intern(Arc::new(result.state));
        let event = self.event.len();
        self.normalization
            .complete(normalization.identity.clone(), event);
        self.event.push(Event {
            identity: normalization.identity,
            target,
            flow: Arc::new(result.flow),
            evidence: BTreeSet::new(),
        });
        self.outgoing[source].push(event);
        for &previous in &self.incoming[source] {
            self.agenda.defer(Task::Compose(previous, event));
        }
        self.support(Atom::State(target), [Atom::Event(event)]);
        for application in normalization.application {
            self.justify(event, application);
        }
    }

    fn justify(&mut self, event: usize, application: Application) {
        let source = self.event[event].identity.source;
        self.event[event].evidence.insert(application.view);
        self.support(
            Atom::Event(event),
            [Atom::State(source), Atom::View(application.view)],
        );
    }
}
