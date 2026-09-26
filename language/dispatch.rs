use crate::catalog::Catalog;
use crate::delta::Delta;
use crate::index::Index;
use crate::mask::Set;
use crate::profile;
use crate::program::Symbol;
use crate::slot::Slot;
use entry::Entry;
use hashing::Builder;
use key::Key;
use smallvec::SmallVec;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::task::Poll;

mod consumer;
mod dependency;
mod entry;
mod key;
mod membership;
mod registry;
mod subscription;

pub(crate) struct Delivery {
    pub rule: usize,
    pub frame: usize,
    pub owner: usize,
    pub read: Option<crate::reader::Read>,
    pub selection: Vec<Slot>,
}

pub(crate) struct Network {
    catalog: Catalog,
    membership: membership::Index,
    sharing: std::sync::Arc<crate::joining::Store>,
    trigger: HashMap<Symbol, Vec<usize>, Builder>,
    broad: Set,
    missing: Vec<usize>,
    enabled: Set,
    entry: registry::Registry,
    ready: Option<BTreeMap<Key, usize>>,
    agenda: VecDeque<usize>,
    store: crate::arena::Store<Entry>,
    retained: usize,
    storage: usize,
    generation: usize,
    cooldown: usize,
    altered: Set,
    selected: Set,
    demand: Vec<consumer::Request>,
    preparation: usize,
    reuse: usize,
}

impl Network {
    pub fn preparation(&self) -> usize {
        self.preparation
    }

    pub fn reuse(&self) -> usize {
        self.reuse
    }

    #[inline]
    pub fn skip(&mut self, maximum: usize) -> usize {
        if maximum < 2 {
            return 0;
        }
        if self.cooldown > 0 {
            self.cooldown -= 1;
            return 0;
        }
        let count = self.batch(maximum);
        if count == 0 {
            self.cooldown = 31;
        }
        count
    }

    fn batch(&mut self, maximum: usize) -> usize {
        let width = self.agenda.len();
        if width == 0 {
            return 0;
        }
        let mut prefix = 0;
        let mut minimum = usize::MAX;
        for &position in self.agenda.iter().take(maximum) {
            let available = self.store[position].waiting();
            if available == 0 {
                break;
            }
            prefix += 1;
            minimum = minimum.min(available);
        }
        if prefix == 0 {
            return 0;
        }
        let _scope = profile::Scope::new(profile::Phase::Matching);
        let count = if prefix == width {
            minimum.min(maximum / width)
        } else {
            1
        };
        if prefix * count < 2 {
            return 0;
        }
        for &position in self.agenda.iter().take(prefix) {
            assert_eq!(self.store[position].skip(count), count);
        }
        if prefix < width {
            self.agenda.rotate_left(prefix);
        }
        prefix * count
    }

    pub fn new(program: &crate::program::Program, index: &Index) -> Self {
        let catalog = Catalog::new(program);
        let mut trigger: HashMap<_, Vec<_>, Builder> = HashMap::default();
        let mut missing = Vec::new();
        let mut enabled = Set::default();
        let mut broad = Set::default();
        for input in 0..catalog.count() {
            if catalog.input(input).broad() {
                broad.insert(input);
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
        let retained = catalog.retained()
            + broad.len()
            + trigger.len()
            + trigger.values().map(Vec::len).sum::<usize>()
            + missing.len();
        let membership = membership::Index::new(index);
        let mut network = Self {
            membership,
            retained,
            storage: 0,
            store: crate::arena::Store::new(),
            generation: 0,
            cooldown: 0,
            altered: Set::default(),
            selected: Set::default(),
            demand: Vec::new(),
            catalog,
            sharing: std::sync::Arc::new(crate::joining::Store::new(65_536)),
            trigger,
            broad,
            missing,
            enabled,
            entry: registry::Registry::new(index.state.frame.len()),
            ready: None,
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

    fn reset(&mut self, index: &Index) {
        let _scope = profile::Scope::new(profile::Phase::Restart);
        self.agenda.clear();
        self.cooldown = 0;
        if self.entry.len() == 0 {
            return;
        }
        let mut reset = |&position: &usize| {
            let entry = &mut self.store[position];
            if !entry.viable() {
                return;
            }
            self.storage -= entry.retained();
            entry.reset(index);
            self.agenda.push_back(position);
            self.storage += entry.retained();
        };
        if let Some(ready) = &self.ready {
            ready.values().for_each(&mut reset);
        } else {
            self.entry.value().for_each(reset);
        }
    }

    pub fn advance(&mut self, index: &Index) {
        let _scope = profile::Scope::new(profile::Phase::Dispatch);
        self.generation += 1;
        self.membership.advance(index, &self.catalog);
        self.entry.resize(index.state.frame.len());
        self.altered.clear();
        self.availability(index);
        let delta = index.delta();
        let mut affected = SmallVec::<[usize; 4]>::from_slice(delta.affected.frame());
        affected.extend_from_slice(&delta.invalidated);
        affected.sort_unstable();
        affected.dedup();
        for frame in affected {
            let previous = self.preparation;
            if index.invalidated(frame) {
                self.frame(index, frame, None);
            } else {
                self.change(index, frame, delta);
            }
            self.refresh(index, frame);
            self.reuse += self.entry.count(frame) - (self.preparation - previous);
        }
        self.reset(index);
    }

    fn change(&mut self, index: &Index, frame: usize, delta: &Delta) {
        let mut selected = std::mem::take(&mut self.selected);
        selected.clone_from(&self.altered);
        if delta.affected.contains(frame) {
            selected.union(&self.broad);
            for symbol in delta.affected.symbol(frame) {
                if let Symbol::Rule(rule) = symbol {
                    selected.insert(self.catalog.rule(rule));
                }
                if delta.toggled.contains(&symbol) {
                    continue;
                }
                let Some(input) = self.trigger.get(&symbol) else {
                    continue;
                };
                for &input in input {
                    if self.enabled.contains(input) {
                        selected.insert(input);
                    }
                }
            }
        }
        if !selected.is_empty() {
            self.frame(index, frame, Some(&selected));
        }
        self.selected = selected;
    }

    #[inline]
    pub fn next(&mut self, index: &Index) -> Poll<Option<Delivery>> {
        let _scope = profile::Scope::new(profile::Phase::Matching);
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
        for &position in self.entry.value() {
            let entry = &mut self.store[position];
            self.storage -= entry.retained();
            entry.evict();
            self.storage += entry.retained();
        }
        self.sharing.evict();
        previous - self.storage - self.sharing.retained() + self.membership.evict()
    }

    #[inline]
    pub fn retained(&self) -> usize {
        self.retained
            + self.membership.retained()
            + self.sharing.retained()
            + self.altered.len()
            + self.enabled.len()
            + self.agenda.len()
            + self.store.retained()
            + self.ready.as_ref().map_or(0, BTreeMap::len)
            + self.entry.retained()
            + self.storage
    }
}

#[cfg(test)]
#[path = "test/dispatch.rs"]
mod test;
