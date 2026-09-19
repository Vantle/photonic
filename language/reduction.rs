use crate::basis::Set;
use crate::flow::{Binding, Place};
use crate::matching;
use crate::program::Program;
use crate::runtime::Limit;
use crate::state::State;
use std::collections::VecDeque;
use std::sync::Arc;
use std::task::Poll;

struct Candidate {
    rule: usize,
    frame: usize,
    owner: usize,
    read: Option<Place>,
    search: crate::search::Search,
}

pub(crate) struct Event {
    pub state: Arc<State>,
    pub rule: usize,
    pub binding: Binding,
    pub fingerprint: crate::fingerprint::Index,
}

pub(crate) struct Search {
    program: Arc<Program>,
    state: Arc<State>,
    agenda: VecDeque<Candidate>,
    pending: Vec<Event>,
    initialized: bool,
    index: Option<Arc<crate::index::Index>>,
    network: crate::activation::Network,
    fingerprint: crate::fingerprint::Index,
    pub work: usize,
}

impl Search {
    pub(crate) fn new(program: Arc<Program>, state: Arc<State>) -> Self {
        Self {
            network: crate::activation::Network::new(&program),
            fingerprint: crate::fingerprint::Index::new(state.clone()),
            program,
            state,
            agenda: VecDeque::new(),
            pending: Vec::new(),
            initialized: false,
            index: None,
            work: 0,
        }
    }

    fn initialize(&mut self) {
        let index = self
            .index
            .get_or_insert_with(|| {
                let mut index = crate::index::Index::new(self.state.clone());
                for (symbol, present) in index.change() {
                    self.network.change(symbol, present);
                }
                Arc::new(index)
            })
            .clone();
        let mut request = Vec::new();
        let mut declaration = vec![Vec::new(); self.program.scope.len()];
        for &rule in &self.network.enabled {
            if let Some(scope) = self.network.scope[rule] {
                declaration[scope].push(rule);
            }
        }
        for frame in index.frame() {
            let mut owner = Some(frame);
            while let Some(current) = owner {
                for &rule in &declaration[self.state.frame[current].scope] {
                    request.push((rule, frame, current, None));
                }
                owner = self.state.frame[current].lexical;
            }
        }
        for &rule in &self.network.enabled {
            for (site, token, capture) in index.code(rule) {
                request.push((
                    rule,
                    self.state.world[site].frame,
                    capture,
                    Some(Place::World(site, token)),
                ));
            }
        }
        for (rule, frame, owner, read) in request {
            let input = &self.program.rule[rule].input;
            let pattern = if input.is_empty() {
                vec![Vec::new()]
            } else {
                matching::pattern(input, Some(owner))
            };
            let search = crate::search::Search::new(pattern, index.clone(), frame);
            if search.viable() {
                self.agenda.push_back(Candidate {
                    rule,
                    frame,
                    owner,
                    read,
                    search,
                });
            }
        }
        self.initialized = true;
    }

    pub(crate) fn record(&self) -> usize {
        self.agenda
            .iter()
            .map(|candidate| candidate.search.retained() + 1)
            .sum::<usize>()
            + self
                .pending
                .iter()
                .map(|event| event.fingerprint.retained() + 1)
                .sum::<usize>()
            + self.index.as_ref().map_or(0, |index| index.retained())
            + self.fingerprint.retained()
            + self.network.retained()
            + 1
    }

    pub(crate) fn advance(
        &mut self,
        state: Arc<State>,
        removed: &Set<usize>,
        fingerprint: crate::fingerprint::Index,
    ) {
        self.agenda.clear();
        self.pending.clear();
        let mut index = Arc::try_unwrap(self.index.take().unwrap()).ok().unwrap();
        index.advance(state.clone(), removed);
        for (symbol, present) in index.change() {
            self.network.change(symbol, present);
        }
        self.index = Some(Arc::new(index));
        self.state = state;
        self.fingerprint = fingerprint;
        self.initialized = false;
    }

    pub(crate) fn run(&mut self, limit: Limit) -> Option<Event> {
        if let Some(index) = self.pending.iter().position(|event| {
            event.state.world.len() <= limit.world
                && event.state.size() <= limit.cell
                && event.state.reachable().len() <= limit.frame
        }) {
            return Some(self.pending.remove(index));
        }
        if !self.initialized {
            self.initialize();
            self.work += 1;
            return None;
        }
        let mut candidate = self.agenda.pop_front()?;
        self.work += 1;
        let selection = match candidate.search.step() {
            Poll::Ready(None) => return None,
            Poll::Pending => {
                self.agenda.push_back(candidate);
                return None;
            }
            Poll::Ready(Some(selection)) => selection,
        };
        if let Some(Place::World(site, _)) = candidate.read
            && !selection.iter().any(|slot| slot.world == site)
        {
            self.agenda.push_back(candidate);
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
            read: candidate.read.into_iter().collect(),
        };
        let result = crate::application::direct(
            &self.state,
            candidate.frame,
            candidate.owner,
            &self.program.rule[candidate.rule],
            &binding,
        );
        let state = Arc::new(result.reclaim());
        let fingerprint = self.fingerprint.advance(state.clone(), &binding.world);
        let event = Event {
            state,
            fingerprint,
            rule: candidate.rule,
            binding,
        };
        self.agenda.push_back(candidate);
        if event.state.world.len() > limit.world
            || event.state.size() > limit.cell
            || event.state.reachable().len() > limit.frame
        {
            self.pending.push(event);
            return None;
        }
        Some(event)
    }
}
