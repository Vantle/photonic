use crate::basis::Set;
use crate::flow::{Binding, Place};
use crate::program::Program;
use crate::runtime::Limit;
use crate::state::State;
use std::sync::Arc;
use std::task::Poll;

pub(crate) struct Event {
    pub state: Arc<State>,
    pub change: crate::change::Change,
    pub rule: usize,
    pub binding: Binding,
    pub fingerprint: crate::fingerprint::Index,
}

pub(crate) struct Search {
    recipe: Vec<crate::recipe::Recipe>,
    retained: usize,
    state: Arc<State>,
    pending: Vec<Event>,
    initialized: bool,
    index: crate::index::Index,
    network: crate::dispatch::Network,
    fingerprint: crate::fingerprint::Index,
    pub work: usize,
}

impl Search {
    pub(crate) fn new(program: Arc<Program>, state: Arc<State>) -> Self {
        let index = crate::index::Index::new(state.clone());
        let recipe = program
            .rule
            .iter()
            .map(crate::recipe::Recipe::new)
            .collect::<Vec<_>>();
        let retained = recipe.iter().map(crate::recipe::Recipe::retained).sum();
        Self {
            network: crate::dispatch::Network::new(&program, &index),
            fingerprint: crate::fingerprint::Index::new(state.clone()),
            recipe,
            retained,
            state,
            pending: Vec::new(),
            initialized: false,
            index,
            work: 0,
        }
    }

    pub(crate) fn preparation(&self) -> usize {
        self.network.preparation
    }

    pub(crate) fn reuse(&self) -> usize {
        self.network.reuse
    }

    pub(crate) fn evict(&mut self) -> usize {
        self.network.evict()
            + self.fingerprint.evict()
            + self
                .pending
                .iter_mut()
                .map(|event| event.fingerprint.evict())
                .sum::<usize>()
    }

    pub(crate) fn record(&self) -> usize {
        self.pending
            .iter()
            .map(|event| event.fingerprint.retained() + event.change.retained() + 1)
            .sum::<usize>()
            + self.retained
            + self.index.retained()
            + self.fingerprint.retained()
            + self.network.retained()
            + 1
    }

    pub(crate) fn advance(
        &mut self,
        state: Arc<State>,
        change: &crate::change::Change,
        fingerprint: crate::fingerprint::Index,
    ) {
        self.pending.clear();
        self.index.update(state.clone(), change);
        self.network.advance(&self.index, &self.state, change);
        self.state = state;
        self.fingerprint = fingerprint;
        self.initialized = false;
    }

    pub(crate) fn run(&mut self, limit: Limit) -> Option<Event> {
        if let Some(index) = self.pending.iter().position(|event| {
            event.state.world.len() <= limit.world
                && event.fingerprint.layout.cell <= limit.cell
                && event.fingerprint.layout.reach.frame.len() <= limit.frame
        }) {
            return Some(self.pending.remove(index));
        }
        if !self.initialized {
            self.initialized = true;
            self.work += 1;
            return None;
        }
        let candidate = match self.network.next(&self.index) {
            Poll::Ready(Some(candidate)) => Some(candidate),
            Poll::Ready(None) => return None,
            Poll::Pending => None,
        };
        self.work += 1;
        let candidate = candidate?;
        let selection = candidate.selection;
        if let Some(read) = candidate.read
            && !selection
                .iter()
                .any(|slot| slot.world == self.index.world(read.site))
        {
            return None;
        }
        let footprint = selection
            .iter()
            .flat_map(|slot| {
                slot.token
                    .iter()
                    .map(|&token| Place::World(slot.world, token))
            })
            .collect::<Set<_>>();
        let binding = Binding {
            world: selection.iter().map(|slot| slot.world).collect(),
            exact: footprint.clone(),
            footprint,
            read: candidate
                .read
                .map(|read| Place::World(self.index.world(read.site), read.resource))
                .into_iter()
                .collect(),
        };
        let result = crate::rewrite::apply(crate::rewrite::Request {
            source: &self.state,
            frame: candidate.frame,
            owner: candidate.owner,
            recipe: &self.recipe[candidate.rule],
            binding: &binding,
            layout: &self.fingerprint.layout,
        });
        let state = Arc::new(result.state);
        let fingerprint = self
            .fingerprint
            .advance(state.clone(), &result.change, result.layout);
        let event = Event {
            state,
            change: result.change,
            fingerprint,
            rule: candidate.rule,
            binding,
        };
        if event.state.world.len() > limit.world
            || event.fingerprint.layout.cell > limit.cell
            || event.fingerprint.layout.reach.frame.len() > limit.frame
        {
            self.pending.push(event);
            return None;
        }
        Some(event)
    }
}
