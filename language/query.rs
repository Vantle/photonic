use crate::index::Index;
use crate::program::Program;
use crate::selection::Selection;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Eq, Hash, PartialEq)]
struct Key {
    pattern: usize,
    frame: usize,
    owner: usize,
}

struct Entry {
    selection: Arc<Selection>,
    generation: usize,
    revision: usize,
}

pub(crate) struct Query {
    catalog: crate::catalog::Catalog,
    entry: HashMap<Key, Entry>,
    generation: usize,
    retained: usize,
    pub preparation: usize,
    pub reuse: usize,
}

impl Query {
    pub(crate) fn new(program: &Program) -> Self {
        Self {
            catalog: crate::catalog::Catalog::new(program),
            entry: HashMap::new(),
            generation: 0,
            retained: 0,
            preparation: 0,
            reuse: 0,
        }
    }

    pub(crate) fn select(
        &mut self,
        rule: usize,
        frame: usize,
        owner: usize,
        index: &Index,
    ) -> Arc<Selection> {
        let pattern = self.catalog.rule(rule);
        let plan = self.catalog.input(pattern);
        let revision = self.catalog.revision(pattern);
        let key = Key {
            pattern,
            frame,
            owner: plan.owner(owner),
        };
        if let Some(entry) = self.entry.get_mut(&key) {
            let selection = if entry.revision == revision {
                None
            } else if entry.generation + 1 == self.generation {
                entry.selection.advance(index, frame)
            } else {
                Some(Selection::new(
                    entry.selection.pattern.clone(),
                    index,
                    frame,
                ))
            };
            if let Some(selection) = selection {
                self.preparation += 1;
                entry.selection = Arc::new(selection);
            } else {
                self.reuse += 1;
            }
            entry.generation = self.generation;
            entry.revision = revision;
            return entry.selection.clone();
        }
        self.preparation += 1;
        let selection = Arc::new(Selection::new(plan.pattern(owner), index, frame));
        self.entry.insert(
            key,
            Entry {
                selection: selection.clone(),
                generation: self.generation,
                revision,
            },
        );
        selection
    }

    pub(crate) fn advance(&mut self, index: &Index) {
        self.generation += 1;
        self.catalog
            .advance(index.altered.iter().copied(), index.occupied);
    }

    pub(crate) fn finish(&mut self) {
        self.entry
            .retain(|_, entry| entry.generation == self.generation);
        self.retained = self
            .entry
            .values()
            .map(|entry| 1 + entry.selection.retained())
            .sum();
    }

    pub(crate) fn retained(&self) -> usize {
        self.catalog.retained() + self.retained
    }
}

#[cfg(test)]
#[path = "test/query.rs"]
mod test;
