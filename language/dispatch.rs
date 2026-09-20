use crate::catalog::Catalog;
use crate::index::Index;
use crate::joining::Join;
use crate::program::Symbol;
use crate::slot::Slot;
use crate::state::State;
use smallvec::SmallVec;
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::task::Poll;

struct Consumer {
    rule: usize,
    owner: usize,
    read: Option<(usize, usize)>,
}

struct Entry {
    frame: usize,
    search: Join,
    consumer: SmallVec<[Consumer; 1]>,
    selection: Option<Vec<Slot>>,
    cursor: usize,
    generation: usize,
}

pub(crate) struct Delivery {
    pub rule: usize,
    pub frame: usize,
    pub owner: usize,
    pub read: Option<(usize, usize)>,
    pub selection: Vec<Slot>,
}

pub(crate) struct Network {
    catalog: Catalog,
    trigger: HashMap<Symbol, Vec<usize>>,
    missing: Vec<usize>,
    enabled: BTreeSet<usize>,
    scope: Vec<BTreeSet<usize>>,
    entry: BTreeMap<(usize, usize, usize), usize>,
    agenda: VecDeque<usize>,
    store: crate::arena::Store<Entry>,
    retained: usize,
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
        for input in 0..catalog.count() {
            let symbol = catalog.input(input).symbol().collect::<BTreeSet<_>>();
            missing.push(symbol.len());
            if symbol.is_empty() {
                enabled.insert(input);
            }
            for symbol in symbol {
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
            + trigger.len()
            + trigger.values().map(Vec::len).sum::<usize>()
            + missing.len()
            + scope.len()
            + scope.iter().map(BTreeSet::len).sum::<usize>();
        let mut network = Self {
            retained,
            store: crate::arena::Store::new(),
            generation: 0,
            altered: BTreeSet::new(),
            catalog,
            trigger,
            scope,
            missing,
            enabled,
            entry: BTreeMap::new(),
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

    fn symbol(&mut self, symbol: Symbol, present: bool) {
        for &input in self.trigger.get(&symbol).into_iter().flatten() {
            if present {
                self.missing[input] -= 1;
                if self.missing[input] == 0 {
                    self.enabled.insert(input);
                    self.altered.insert(input);
                    for &owner in self.catalog.owner(input) {
                        if self.scope[owner].insert(input) {
                            self.retained += 1;
                        }
                    }
                }
            } else {
                self.missing[input] += 1;
                if self.enabled.remove(&input) {
                    self.altered.insert(input);
                }
                for &owner in self.catalog.owner(input) {
                    if self.scope[owner].remove(&input) {
                        self.retained -= 1;
                    }
                }
            }
        }
    }

    fn refresh(&mut self, index: &Index, frame: usize) {
        for (&(_, input, _), &position) in self
            .entry
            .range((frame, 0, 0)..=(frame, usize::MAX, usize::MAX))
        {
            let entry = &mut self.store[position];
            if entry.generation == self.generation {
                continue;
            }
            entry.generation = self.generation;
            let plan = self.catalog.input(input);
            if plan.empty()
                || index
                    .affected
                    .get(&frame)
                    .is_some_and(|symbol| plan.symbol().any(|value| symbol.contains(&value)))
            {
                entry.search.advance(index, frame);
                self.preparation += 1;
            } else {
                self.reuse += 1;
            }
        }
    }

    fn frame(&mut self, index: &Index, frame: usize, selected: Option<&BTreeSet<usize>>) {
        let mut request = Vec::new();
        if index.present(frame) {
            let mut owner = Some(frame);
            while let Some(current) = owner {
                let scope = index.state.frame[current].scope;
                for &input in selected.unwrap_or(&self.scope[scope]) {
                    if !self.enabled.contains(&input) {
                        continue;
                    }
                    let plan = self.catalog.input(input);
                    for &rule in self.catalog.scope(scope, input) {
                        request.push((
                            input,
                            plan.owner(current),
                            Consumer {
                                rule,
                                owner: current,
                                read: None,
                            },
                        ));
                    }
                }
                owner = index.state.frame[current].lexical;
            }
            for reader in index.reader(frame) {
                let input = self.catalog.rule(reader.rule);
                if !self.enabled.contains(&input)
                    || selected.is_some_and(|selected| !selected.contains(&input))
                {
                    continue;
                }
                request.push((
                    input,
                    self.catalog.input(input).owner(reader.owner),
                    Consumer {
                        rule: reader.rule,
                        owner: reader.owner,
                        read: Some((reader.site, reader.resource)),
                    },
                ));
            }
        }
        request.sort_by_key(|&(input, owner, _)| (input, owner));
        let removal = self
            .entry
            .range((frame, 0, 0)..=(frame, usize::MAX, usize::MAX))
            .filter(|(key, _)| {
                selected.is_none_or(|selected| selected.contains(&key.1))
                    && request
                        .binary_search_by_key(&(key.1, key.2), |&(input, owner, _)| (input, owner))
                        .is_err()
            })
            .map(|(&key, _)| key)
            .collect::<Vec<_>>();
        for key in removal {
            let position = self.entry.remove(&key).unwrap();
            self.store.remove(position);
        }
        let mut request = request.into_iter().peekable();
        while let Some((input, owner, first)) = request.next() {
            let mut consumer = SmallVec::new();
            consumer.push(first);
            while request
                .peek()
                .is_some_and(|&(next, capture, _)| next == input && capture == owner)
            {
                consumer.push(request.next().unwrap().2);
            }
            let plan = self.catalog.input(input);
            if let Some(&position) = self.entry.get(&(frame, input, owner)) {
                let entry = &mut self.store[position];
                entry.consumer = consumer;
            } else {
                let pattern = plan.pattern(owner);
                let position = self.store.insert(Entry {
                    frame,
                    search: Join::new(pattern, index, frame),
                    consumer,
                    selection: None,
                    cursor: 0,
                    generation: self.generation,
                });
                self.entry.insert((frame, input, owner), position);
                self.preparation += 1;
            }
        }
    }

    fn reset(&mut self) {
        self.agenda.clear();
        for &position in self.entry.values() {
            let entry = &mut self.store[position];
            entry.selection = None;
            entry.cursor = 0;
            if entry.search.viable() {
                entry.search.reset();
                self.agenda.push_back(position);
            }
        }
    }

    pub fn advance(&mut self, index: &Index, previous: &State, change: &crate::change::Change) {
        self.generation += 1;
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
        }
        self.reset();
    }

    pub fn next(&mut self, index: &Index) -> Option<Poll<Delivery>> {
        let position = self.agenda.pop_front()?;
        let entry = &mut self.store[position];
        if entry.selection.is_none() {
            match entry.search.step(index) {
                Poll::Ready(Some(selection)) => {
                    entry.selection = Some(selection);
                    entry.cursor = 0;
                }
                Poll::Ready(None) => return Some(Poll::Pending),
                Poll::Pending => {
                    self.agenda.push_back(position);
                    return Some(Poll::Pending);
                }
            }
        }
        let consumer = &entry.consumer[entry.cursor];
        entry.cursor += 1;
        let selection = if entry.cursor == entry.consumer.len() {
            entry.selection.take().unwrap()
        } else {
            entry.selection.as_ref().unwrap().clone()
        };
        let delivery = Delivery {
            rule: consumer.rule,
            frame: entry.frame,
            owner: consumer.owner,
            read: consumer.read,
            selection,
        };
        self.agenda.push_back(position);
        Some(Poll::Ready(delivery))
    }

    pub fn retained(&self) -> usize {
        self.retained
            + self.altered.len()
            + self.enabled.len()
            + self.agenda.len()
            + self.store.retained()
            + self
                .store
                .iter()
                .map(|entry| {
                    entry.search.retained()
                        + entry.consumer.len()
                        + entry.selection.as_ref().map_or(0, |selection| {
                            selection
                                .iter()
                                .map(|slot| slot.token.len() + 1)
                                .sum::<usize>()
                        })
                        + 1
                })
                .sum::<usize>()
    }
}

#[cfg(test)]
#[path = "test/dispatch.rs"]
mod test;
