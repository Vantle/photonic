use crate::catalog::Catalog;
use crate::index::Index;
use crate::membership::Set;
use crate::program::Symbol;
use crate::replay::Search;
use crate::slot::Slot;
use crate::state::State;
use entry::Entry;
use key::Key;
use smallvec::SmallVec;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::task::Poll;

mod consumer;
mod context;
mod dependency;
mod entry;
mod key;

pub(crate) struct Delivery {
    pub rule: usize,
    pub frame: usize,
    pub owner: usize,
    pub read: Option<crate::reader::Read>,
    pub selection: Vec<Slot>,
}

pub(crate) struct Network {
    catalog: Catalog,
    sharing: std::sync::Arc<crate::joining::Store>,
    trigger: HashMap<Symbol, Vec<usize>>,
    empty: Vec<usize>,
    missing: Vec<usize>,
    enabled: Set,
    scope: Vec<Set>,
    entry: BTreeMap<Key, usize>,
    ready: Option<BTreeMap<Key, usize>>,
    count: Vec<usize>,
    agenda: VecDeque<usize>,
    store: crate::arena::Store<Entry>,
    retained: usize,
    storage: usize,
    generation: usize,
    altered: Set,
    pub preparation: usize,
    pub reuse: usize,
}

impl Network {
    pub fn new(program: &crate::program::Program, index: &Index) -> Self {
        let catalog = Catalog::new(program);
        let mut trigger: HashMap<_, Vec<_>> = HashMap::new();
        let mut missing = Vec::new();
        let mut enabled = Set::default();
        let mut empty = Vec::new();
        for input in 0..catalog.count() {
            if catalog.input(input).empty() {
                empty.push(input);
            }
            let symbol = catalog.input(input).dependency();
            missing.push(symbol.len());
            if symbol.is_empty() {
                enabled.insert(input);
            }
            for &symbol in symbol {
                trigger.entry(symbol).or_default().push(input);
            }
        }
        let mut scope = vec![Set::default(); program.scope.len()];
        for &input in &enabled {
            for &owner in catalog.owner(input) {
                scope[owner].insert(input);
            }
        }
        let retained = catalog.retained()
            + empty.len()
            + trigger.len()
            + trigger.values().map(Vec::len).sum::<usize>()
            + missing.len()
            + scope.len()
            + scope.iter().map(Set::len).sum::<usize>();
        let mut network = Self {
            retained,
            storage: 0,
            store: crate::arena::Store::new(),
            generation: 0,
            altered: Set::default(),
            catalog,
            sharing: std::sync::Arc::new(crate::joining::Store::new(65_536)),
            trigger,
            empty,
            scope,
            missing,
            enabled,
            entry: BTreeMap::new(),
            ready: None,
            count: vec![0; index.state.frame.len()],
            agenda: VecDeque::new(),
            preparation: 0,
            reuse: 0,
        };
        for symbol in index.available() {
            network.symbol(symbol, true);
        }
        for frame in index.frame() {
            network.frame(index, frame, None);
        }
        network.reset(index);
        network
    }

    fn frame(&mut self, index: &Index, frame: usize, selected: Option<&Set>) {
        #[cfg(feature = "measurement")]
        let _measurement = crate::measurement::profile::Scope::new(
            crate::measurement::profile::Phase::Subscription,
        );
        let request = self.request(index, frame, selected);
        let interval = selected
            .filter(|selected| selected.len() < self.count.get(frame).copied().unwrap_or(0))
            .map_or_else(
                || vec![Key::frame(frame)],
                |selected| {
                    selected
                        .iter()
                        .map(|&input| Key::input(frame, input))
                        .collect()
                },
            );
        let removal = interval
            .into_iter()
            .flat_map(|interval| self.entry.range(interval))
            .filter(|(key, _)| {
                selected.is_none_or(|selected| selected.contains(&key.input))
                    && request
                        .binary_search_by_key(*key, |request| request.key)
                        .is_err()
            })
            .map(|(&key, _)| key)
            .collect::<Vec<_>>();
        for key in removal {
            let position = self.entry.remove(&key).unwrap();
            if let Some(ready) = &mut self.ready {
                ready.remove(&key);
            }
            self.count[frame] -= 1;
            self.storage -= self.store.remove(position).retained();
        }
        let mut request = request.into_iter().peekable();
        while let Some(first) = request.next() {
            let key = first.key;
            let mut consumer = SmallVec::new();
            consumer.push(first.consumer);
            while request.peek().is_some_and(|next| next.key == key) {
                consumer.push(request.next().unwrap().consumer);
            }
            let plan = self.catalog.input(key.input);
            if let Some(&position) = self.entry.get(&key) {
                let entry = &mut self.store[position];
                self.storage -= entry.retained();
                entry.replace(consumer);
                self.storage += entry.retained();
            } else {
                let entry = Entry::new(
                    Search::planned(crate::joining::Request {
                        input: plan,
                        index,
                        frame,
                        owner: key.owner,
                        store: &self.sharing,
                    }),
                    consumer,
                    self.generation,
                );
                self.storage += entry.retained();
                let viable = entry.viable();
                let position = self.store.insert(entry);
                self.entry.insert(key, position);
                if let Some(ready) = &mut self.ready {
                    if viable {
                        ready.insert(key, position);
                    }
                } else if self.entry.len() > 32 {
                    self.ready = Some(
                        self.entry
                            .iter()
                            .filter(|&(_, &position)| self.store[position].viable())
                            .map(|(&key, &position)| (key, position))
                            .collect(),
                    );
                }
                self.count[frame] += 1;
                self.preparation += 1;
            }
        }
    }

    fn reset(&mut self, index: &Index) {
        #[cfg(feature = "measurement")]
        let _measurement =
            crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Restart);
        self.agenda.clear();
        for &position in self.ready.as_ref().unwrap_or(&self.entry).values() {
            let entry = &mut self.store[position];
            if !entry.viable() {
                continue;
            }
            self.storage -= entry.retained();
            entry.reset(index);
            self.agenda.push_back(position);
            self.storage += entry.retained();
        }
    }

    pub fn advance(&mut self, index: &Index, previous: &State, change: &crate::change::Change) {
        #[cfg(feature = "measurement")]
        let _measurement =
            crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Dispatch);
        self.generation += 1;
        self.count
            .resize(self.count.len().max(index.state.frame.len()), 0);
        self.altered.clear();
        self.availability(index);
        let mut affected = index
            .affected
            .keys()
            .copied()
            .collect::<SmallVec<[usize; 4]>>();
        let mut changed = change
            .frame
            .iter()
            .copied()
            .filter(
                |&frame| match (previous.frame.get(frame), index.state.frame.get(frame)) {
                    (Some(left), Some(right)) => {
                        left.scope != right.scope || left.lexical != right.lexical
                    }
                    _ => true,
                },
            )
            .collect::<SmallVec<[usize; 4]>>();
        changed.sort_unstable();
        changed.dedup();
        let mut context = changed.clone();
        context.extend_from_slice(&index.context);
        affected.extend_from_slice(&changed);
        if !changed.is_empty() {
            let descendant = context::select(index, &changed);
            affected.extend_from_slice(&descendant);
            context.extend_from_slice(&descendant);
        }
        affected.sort_unstable();
        affected.dedup();
        context.sort_unstable();
        context.dedup();
        for frame in affected {
            let previous = self.preparation;
            if context.binary_search(&frame).is_ok() {
                self.frame(index, frame, None);
            } else {
                let mut selected = self.altered.clone();
                if let Some(symbol) = index.affected.get(&frame) {
                    for symbol in symbol {
                        if let Symbol::Rule(rule) = *symbol {
                            selected.insert(self.catalog.rule(rule));
                        }
                    }
                }
                if !selected.is_empty() {
                    self.frame(index, frame, Some(&selected));
                }
            }
            self.refresh(index, frame);
            self.reuse +=
                self.count.get(frame).copied().unwrap_or(0) - (self.preparation - previous);
        }
        self.reset(index);
    }

    #[inline]
    pub fn next(&mut self, index: &Index) -> Poll<Option<Delivery>> {
        #[cfg(feature = "measurement")]
        let _measurement =
            crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Matching);
        let Some(position) = self.agenda.pop_front() else {
            return Poll::Ready(None);
        };
        let entry = &mut self.store[position];
        self.storage -= entry.retained();
        let delivery = entry.next(index);
        self.storage += entry.retained();
        match delivery {
            Poll::Ready(None) => Poll::Pending,
            Poll::Ready(Some(delivery)) => {
                self.agenda.push_back(position);
                Poll::Ready(Some(delivery))
            }
            Poll::Pending => {
                self.agenda.push_back(position);
                Poll::Pending
            }
        }
    }

    pub fn evict(&mut self) -> usize {
        let previous = self.storage + self.sharing.retained();
        for &position in self.entry.values() {
            let entry = &mut self.store[position];
            self.storage -= entry.retained();
            entry.evict();
            self.storage += entry.retained();
        }
        self.sharing.evict();
        previous - self.storage - self.sharing.retained()
    }

    #[inline]
    pub fn retained(&self) -> usize {
        self.retained
            + self.sharing.retained()
            + self.altered.len()
            + self.enabled.len()
            + self.agenda.len()
            + self.store.retained()
            + self.ready.as_ref().map_or(0, BTreeMap::len)
            + self.count.len()
            + self.storage
    }
}

#[cfg(test)]
#[path = "test/dispatch.rs"]
mod test;
