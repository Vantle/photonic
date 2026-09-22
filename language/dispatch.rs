use crate::catalog::Catalog;
use crate::index::Index;
use crate::membership::Set;
use crate::program::Symbol;
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
    sharing: std::sync::Arc<crate::joining::Store>,
    trigger: HashMap<Symbol, Vec<usize>>,
    empty: Vec<usize>,
    missing: Vec<usize>,
    enabled: Set,
    scope: Vec<crate::bitmap::Set>,
    entry: registry::Registry,
    ready: Option<BTreeMap<Key, usize>>,
    agenda: VecDeque<usize>,
    store: crate::arena::Store<Entry>,
    retained: usize,
    storage: usize,
    generation: usize,
    cooldown: usize,
    altered: Set,
    context: context::Context,
    pub preparation: usize,
    pub reuse: usize,
}

impl Network {
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
        #[cfg(feature = "measurement")]
        let _measurement =
            crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Matching);
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
        let mut scope = (0..program.scope.len())
            .map(|scope| crate::bitmap::Set::new(catalog.width(scope)))
            .collect::<Vec<_>>();
        for &input in &enabled {
            for owner in catalog.owner(input) {
                scope[owner.scope].insert(owner.position);
            }
        }
        let retained = catalog.retained()
            + empty.len()
            + trigger.len()
            + trigger.values().map(Vec::len).sum::<usize>()
            + missing.len()
            + scope.len()
            + scope.iter().map(crate::bitmap::Set::len).sum::<usize>();
        let mut network = Self {
            retained,
            storage: 0,
            store: crate::arena::Store::new(),
            generation: 0,
            cooldown: 0,
            altered: Set::default(),
            context: context::Context::default(),
            catalog,
            sharing: std::sync::Arc::new(crate::joining::Store::new(65_536)),
            trigger,
            empty,
            scope,
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
        #[cfg(feature = "measurement")]
        let _measurement =
            crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Restart);
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
            self.entry.values().for_each(reset);
        }
    }

    pub fn advance(&mut self, index: &Index, previous: &State, change: &crate::change::Change) {
        #[cfg(feature = "measurement")]
        let _measurement =
            crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Dispatch);
        self.generation += 1;
        self.entry.resize(index.state.frame.len());
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
            let descendant = self.context.select(index, previous, &changed);
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
            self.reuse += self.entry.count(frame) - (self.preparation - previous);
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
        let previous = self.storage + self.sharing.retained() + self.context.retained();
        for &position in self.entry.values() {
            let entry = &mut self.store[position];
            self.storage -= entry.retained();
            entry.evict();
            self.storage += entry.retained();
        }
        self.sharing.evict();
        self.context.evict();
        previous - self.storage - self.sharing.retained()
    }

    #[inline]
    pub fn retained(&self) -> usize {
        self.retained
            + self.context.retained()
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
