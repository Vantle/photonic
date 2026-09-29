use super::taxonomy::Makeup;
use super::{map, shard, update};
use crate::executor::Executor;
use hashing::Builder;
use indexmap::{Equivalent, IndexMap};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

struct Entry {
    hash: u64,
    makeup: Arc<Makeup>,
}

#[derive(Clone, Copy)]
struct Probe<'makeup> {
    hash: u64,
    makeup: &'makeup Makeup,
}

impl Hash for Entry {
    fn hash<Target: Hasher>(&self, hasher: &mut Target) {
        hasher.write_u64(self.hash);
    }
}

impl PartialEq for Entry {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.makeup == other.makeup
    }
}

impl Eq for Entry {}

impl Hash for Probe<'_> {
    fn hash<Target: Hasher>(&self, hasher: &mut Target) {
        hasher.write_u64(self.hash);
    }
}

impl PartialEq for Probe<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash && self.makeup == other.makeup
    }
}

impl Eq for Probe<'_> {}

impl Equivalent<Entry> for Probe<'_> {
    fn equivalent(&self, key: &Entry) -> bool {
        self.hash == key.hash && *self.makeup == *key.makeup
    }
}

pub(super) enum Found {
    Existing(usize),
    New,
    Repeat(usize),
}

// Configurations are found by their makeup, hashed where it is made, so shards find them in
// parallel and only numbering the new ones happens in order.
pub(super) struct Space {
    table: Vec<IndexMap<Entry, usize, Builder>>,
}

impl Default for Space {
    fn default() -> Self {
        Self {
            table: shard::empty(),
        }
    }
}

pub(super) fn hash(makeup: &Makeup) -> u64 {
    hashing::value(makeup)
}

impl Space {
    pub fn find(&self, makeup: &Makeup) -> Option<usize> {
        let hash = hash(makeup);
        self.table[shard::slot(hash)]
            .get(&Probe { hash, makeup })
            .copied()
    }

    pub fn resolve(&self, executor: Option<&Executor>, item: &[(u64, &Makeup)]) -> Vec<Found> {
        let mut group = shard::empty::<Vec<_>>();
        for (position, &(hash, _)) in item.iter().enumerate() {
            group[shard::slot(hash)].push(position);
        }
        group.retain(|list| !list.is_empty());
        let resolved = map(executor, group, |list| {
            let mut first = IndexMap::<Probe<'_>, usize, Builder>::default();
            list.into_iter()
                .map(|position| {
                    let (hash, makeup) = item[position];
                    let probe = Probe { hash, makeup };
                    if let Some(&index) = self.table[shard::slot(hash)].get(&probe) {
                        return (position, Found::Existing(index));
                    }
                    match first.get(&probe) {
                        Some(&earlier) => (position, Found::Repeat(earlier)),
                        None => {
                            first.insert(probe, position);
                            (position, Found::New)
                        }
                    }
                })
                .collect::<Vec<_>>()
        });
        let mut found = (0..item.len()).map(|_| Found::New).collect::<Vec<_>>();
        for (position, value) in resolved.into_iter().flatten() {
            found[position] = value;
        }
        found
    }

    pub fn admit(&mut self, executor: Option<&Executor>, item: Vec<(u64, Arc<Makeup>, usize)>) {
        let mut group = shard::empty::<Vec<_>>();
        for (hash, makeup, index) in item {
            group[shard::slot(hash)].push((Entry { hash, makeup }, index));
        }
        let group = group
            .into_iter()
            .enumerate()
            .filter(|(_, list)| !list.is_empty())
            .collect();
        update(executor, &mut self.table, group, |table, list| {
            table.extend(list);
        });
    }
}
