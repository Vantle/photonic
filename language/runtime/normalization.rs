use super::{Application, Identity};
use crate::canonical::Search;
use crate::flow::{Applied, Flow};
use crate::hashing::Builder;
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub(super) enum Status {
    Complete(usize),
    Pending(usize),
}

struct Job {
    identity: Identity,
    application: Vec<Application>,
    flow: Flow,
    search: Option<Search>,
}

pub(super) struct Completion {
    pub identity: Identity,
    pub application: Vec<Application>,
    pub result: Applied,
}

#[derive(Default)]
pub(super) struct Store {
    environment: super::environment::Store,
    identity: HashMap<Identity, Status, Builder>,
    job: crate::arena::Store<Job>,
}

impl Store {
    pub fn environment(
        &mut self,
        request: super::environment::Request<'_>,
    ) -> std::sync::Arc<crate::state::State> {
        self.environment.resolve(request)
    }

    pub fn find(&self, identity: &Identity) -> Option<Status> {
        self.identity.get(identity).copied()
    }

    pub fn attach(&mut self, index: usize, application: Application) {
        self.job[index].application.push(application);
    }

    pub fn insert(
        &mut self,
        identity: Identity,
        application: Application,
        result: Applied,
    ) -> usize {
        let index = self.job.insert(Job {
            identity: identity.clone(),
            application: vec![application],
            flow: result.flow,
            search: Some(Search::new(std::sync::Arc::new(result.state))),
        });
        self.identity.insert(identity, Status::Pending(index));
        index
    }

    pub fn take(&mut self, index: usize) -> Search {
        self.job[index].search.take().unwrap()
    }

    pub fn advance(&mut self, index: usize, search: Search, complete: bool) -> Option<Completion> {
        if !complete {
            self.job[index].search = Some(search);
            return None;
        }
        let job = self.job.remove(index);
        self.identity.remove(&job.identity);
        Some(Completion {
            identity: job.identity,
            application: job.application,
            result: job.flow.rename(search.finish().unwrap()),
        })
    }

    pub fn complete(&mut self, identity: Identity, event: usize) {
        self.identity.insert(identity, Status::Complete(event));
    }

    pub fn retained(&self) -> usize {
        self.job.retained()
    }
}
