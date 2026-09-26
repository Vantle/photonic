use crate::flow::Binding;
use crate::layout::Layout;
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
    pub layout: Layout,
}

impl Event {
    fn admitted(&self, limit: Limit) -> bool {
        limit.admits(
            self.state.world.len(),
            self.layout.cell,
            self.layout.reach.frame.len(),
        )
    }

    #[inline]
    pub(crate) fn retained(&self) -> usize {
        self.fingerprint.retained() + self.layout.reach.retained()
    }

    pub(crate) fn evict(&mut self) -> usize {
        self.fingerprint.evict() + self.layout.reach.evict()
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
    layout: Layout,
    work: usize,
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
        let layout = Layout::new(&state);
        let fingerprint = crate::fingerprint::Index::new(state.clone(), &layout.reach.frame);
        let index = crate::index::Index::prepared(state.clone(), layout.reach.frame.clone());
        Self {
            network: crate::dispatch::Network::new(&program, &index),
            fingerprint,
            layout,
            program,
            state,
            pending: Vec::new(),
            initialized: false,
            index,
            work: 0,
        }
    }

    #[inline]
    pub(crate) fn work(&self) -> usize {
        self.work
    }

    pub(crate) fn deferred(&self) -> usize {
        self.pending.len()
    }

    pub(crate) fn preparation(&self) -> usize {
        self.network.preparation()
    }

    pub(crate) fn reuse(&self) -> usize {
        self.network.reuse()
    }

    pub(crate) fn evict(&mut self) -> usize {
        self.network.evict()
            + self.fingerprint.evict()
            + self.layout.reach.evict()
            + self.pending.iter_mut().map(Event::evict).sum::<usize>()
    }

    #[inline]
    pub(crate) fn record(&self) -> usize {
        self.pending
            .iter()
            .map(|event| event.retained() + event.change.retained() + 1)
            .sum::<usize>()
            + self.index.retained()
            + self.fingerprint.retained()
            + self.layout.reach.retained()
            + self.network.retained()
            + 1
    }

    pub(crate) fn advance(
        &mut self,
        state: Arc<State>,
        change: &crate::change::Change,
        fingerprint: crate::fingerprint::Index,
        layout: Layout,
    ) {
        self.pending.clear();
        self.index
            .apply(state.clone(), change, layout.reach.frame.clone());
        self.network.advance(&self.index);
        self.state = state;
        self.fingerprint = fingerprint;
        self.layout = layout;
        self.initialized = false;
    }

    pub(crate) fn run(&mut self, limit: Limit) -> Poll<Option<Event>> {
        if let Some(index) = self.pending.iter().position(|event| event.admitted(limit)) {
            return Poll::Ready(Some(self.pending.remove(index)));
        }
        if !self.initialized {
            self.initialized = true;
            self.work += 1;
            return Poll::Pending;
        }
        let candidate = match self.network.next(&self.index) {
            Poll::Ready(Some(candidate)) => candidate,
            Poll::Ready(None) => return Poll::Ready(None),
            Poll::Pending => {
                self.work += 1;
                return Poll::Pending;
            }
        };
        self.work += 1;
        let selection = candidate.selection;
        if let Some(crate::reader::Read::World(site, _)) = candidate.read
            && !crate::slot::admits(&selection, self.index.world(site))
        {
            return Poll::Pending;
        }
        let read = candidate
            .read
            .map(|read| read.place(&self.index))
            .into_iter()
            .collect();
        let Some(binding) = Binding::select(&self.state, &selection, candidate.frame, read) else {
            return Poll::Pending;
        };
        let result = crate::evaluation::apply(crate::evaluation::Request {
            source: &self.state,
            frame: candidate.frame,
            owner: candidate.owner,
            rule: &self.program.rule[candidate.rule],
            scope: &self.program.scope,
            state: self.state.as_ref().clone(),
            next: self.layout.resource,
            binding: &binding,
            layout: &self.layout,
        });
        let state = Arc::new(result.state);
        let fingerprint =
            self.fingerprint
                .advance(state.clone(), &result.change, &result.layout.reach.frame);
        let event = Event {
            state,
            change: result.change,
            fingerprint,
            layout: result.layout,
            rule: candidate.rule,
            binding,
        };
        if !event.admitted(limit) {
            self.pending.push(event);
            return Poll::Pending;
        }
        Poll::Ready(Some(event))
    }
}
