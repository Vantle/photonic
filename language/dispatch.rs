use crate::catalog::Catalog;
use crate::index::Index;
use crate::program::Symbol;
use crate::replay::Search;
use crate::slot::Slot;
use crate::state::State;
use entry::Entry;
use key::Key;
use smallvec::SmallVec;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::task::Poll;

mod consumer;
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
    trigger: HashMap<Symbol, Vec<usize>>,
    empty: Vec<usize>,
    missing: Vec<usize>,
    enabled: BTreeSet<usize>,
    scope: Vec<BTreeSet<usize>>,
    entry: BTreeMap<Key, usize>,
    ready: Option<BTreeMap<Key, usize>>,
    count: Vec<usize>,
    agenda: VecDeque<usize>,
    store: crate::arena::Store<Entry>,
    retained: usize,
    storage: usize,
    generation: usize,
    altered: BTreeSet<usize>,
    pub preparation: usize,
    pub reuse: usize,
}

impl Network {
    pub fn new(program: &crate::program::Program, index: &Index) -> Self {
        let catalog = Catalog::new(program);
        let mut trigger: HashMap<_, Vec<_>> = HashMap::new();
        let mut missing = Vec::new();
        let mut enabled = BTreeSet::new();
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
        let mut scope = vec![BTreeSet::new(); program.scope.len()];
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
            + scope.iter().map(BTreeSet::len).sum::<usize>();
        let mut network = Self {
            retained,
            storage: 0,
            store: crate::arena::Store::new(),
            generation: 0,
            altered: BTreeSet::new(),
            catalog,
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
        network.reset();
        network
    }

    fn frame(&mut self, index: &Index, frame: usize, selected: Option<&BTreeSet<usize>>) {
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
                let pattern = plan.pattern(key.owner);
                let entry = Entry::new(
                    Search::new(pattern, index, frame),
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

    fn reset(&mut self) {
        self.agenda.clear();
        for &position in self.ready.as_ref().unwrap_or(&self.entry).values() {
            let entry = &mut self.store[position];
            if !entry.viable() {
                continue;
            }
            self.storage -= entry.retained();
            entry.reset();
            self.agenda.push_back(position);
            self.storage += entry.retained();
        }
    }

    pub fn advance(&mut self, index: &Index, previous: &State, change: &crate::change::Change) {
        self.generation += 1;
        self.count
            .resize(self.count.len().max(index.state.frame.len()), 0);
        self.altered.clear();
        for &symbol in &index.altered {
            self.symbol(symbol, index.contains(&symbol));
        }
        let mut affected = index.affected.keys().copied().collect::<BTreeSet<_>>();
        let changed = change
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
            .collect::<BTreeSet<_>>();
        let mut context = changed.clone();
        context.extend(&index.context);
        affected.extend(&changed);
        if !changed.is_empty() {
            for frame in index.frame() {
                let mut owner = Some(frame);
                while let Some(current) = owner {
                    if changed.contains(&current) {
                        affected.insert(frame);
                        context.insert(frame);
                        break;
                    }
                    owner = index.state.frame[current].lexical;
                }
            }
        }
        for frame in affected {
            let previous = self.preparation;
            if context.contains(&frame) {
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
        self.reset();
    }

    pub fn next(&mut self, index: &Index) -> Poll<Option<Delivery>> {
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

    pub fn retained(&self) -> usize {
        self.retained
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
