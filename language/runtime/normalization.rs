use super::{Application, Identity};
use crate::canonical::Search;
use crate::flow::{Applied, Flow};
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
    identity: HashMap<Identity, Status>,
    job: Vec<Option<Job>>,
    vacant: Vec<usize>,
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
        self.job[index]
            .as_mut()
            .unwrap()
            .application
            .push(application);
    }

    pub fn insert(
        &mut self,
        identity: Identity,
        application: Application,
        result: Applied,
    ) -> usize {
        let index = self.vacant.pop().unwrap_or_else(|| {
            let index = self.job.len();
            self.job.push(None);
            index
        });
        self.identity
            .insert(identity.clone(), Status::Pending(index));
        self.job[index] = Some(Job {
            identity,
            application: vec![application],
            flow: result.flow,
            search: Some(Search::new(std::sync::Arc::new(result.state))),
        });
        index
    }

    pub fn take(&mut self, index: usize) -> Search {
        self.job[index].as_mut().unwrap().search.take().unwrap()
    }

    pub fn advance(&mut self, index: usize, search: Search, complete: bool) -> Option<Completion> {
        if !complete {
            self.job[index].as_mut().unwrap().search = Some(search);
            return None;
        }
        let job = self.job[index].take().unwrap();
        self.vacant.push(index);
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
        self.job.len() + self.vacant.len()
    }
}
