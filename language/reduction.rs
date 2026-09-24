use crate::flow::Binding;
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

impl Event {
    fn admitted(&self, limit: Limit) -> bool {
        limit.admits(
            self.state.world.len(),
            self.fingerprint.layout.cell,
            self.fingerprint.layout.reach.frame.len(),
        )
    }
}

pub(crate) struct Search {
    program: Arc<Program>,
    state: Arc<State>,
    pending: Vec<Event>,
    initialized: bool,
    index: crate::index::Index,
    network: crate::dispatch::Network,
    fingerprint: crate::fingerprint::Index,
    pub work: usize,
}

impl Search {
    pub(crate) fn skip(&mut self, maximum: usize) -> usize {
        if !self.initialized || !self.pending.is_empty() {
            return 0;
        }
        let count = self.network.skip(maximum);
        self.work += count;
        count
    }

    pub(crate) fn new(program: Arc<Program>, state: Arc<State>) -> Self {
        let fingerprint = crate::fingerprint::Index::new(state.clone());
        let index = crate::index::Index::prepared(state.clone(), fingerprint.layout.reach.clone());
        Self {
            network: crate::dispatch::Network::new(&program, &index),
            fingerprint,
            program,
            state,
            pending: Vec::new(),
            initialized: false,
            index,
            work: 0,
        }
    }

    pub(crate) fn deferred(&self) -> usize {
        self.pending.len()
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

    #[inline]
    pub(crate) fn record(&self) -> usize {
        self.pending
            .iter()
            .map(|event| event.fingerprint.retained() + event.change.retained() + 1)
            .sum::<usize>()
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
        self.index
            .apply(state.clone(), change, fingerprint.layout.reach.clone());
        self.network.advance(&self.index);
        self.state = state;
        self.fingerprint = fingerprint;
        self.initialized = false;
    }

    pub(crate) fn run(&mut self, limit: Limit) -> Option<Event> {
        if let Some(index) = self.pending.iter().position(|event| event.admitted(limit)) {
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
        if let Some(crate::reader::Read::World(site, _)) = candidate.read
            && !crate::slot::admits(&selection, self.index.world(site))
        {
            return None;
        }
        let mut binding = Binding::select(&self.state, &selection)?;
        binding.read = candidate
            .read
            .map(|read| read.place(&self.index))
            .into_iter()
            .collect();
        let result = crate::evaluation::apply(crate::evaluation::Request {
            source: &self.state,
            frame: candidate.frame,
            owner: candidate.owner,
            rule: &self.program.rule[candidate.rule],
            scope: &self.program.scope,
            state: self.state.as_ref().clone(),
            next: self.fingerprint.layout.resource,
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
        if !event.admitted(limit) {
            self.pending.push(event);
            return None;
        }
        Some(event)
    }
}
