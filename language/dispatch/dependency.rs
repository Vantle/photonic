use super::{Key, Network};
use crate::index::Index;
use crate::program::Symbol;
use smallvec::SmallVec;
use std::collections::BTreeSet;

impl Network {
    pub(super) fn availability(&mut self, index: &Index) {
        #[cfg(feature = "measurement")]
        let _measurement = crate::measurement::profile::Scope::new(
            crate::measurement::profile::Phase::Availability,
        );
        let mut insertion = SmallVec::<[Symbol; 4]>::new();
        for &symbol in &index.altered {
            if index.contains(&symbol) {
                insertion.push(symbol);
            } else {
                self.symbol(symbol, false);
            }
        }
        for symbol in insertion {
            self.symbol(symbol, true);
        }
    }

    pub(super) fn symbol(&mut self, symbol: Symbol, present: bool) {
        for &input in self.trigger.get(&symbol).into_iter().flatten() {
            if present {
                self.missing[input] -= 1;
                if self.missing[input] == 0 {
                    self.enabled.insert(input);
                    self.altered.insert(input);
                }
            } else {
                self.missing[input] += 1;
                if self.missing[input] != 1 {
                    continue;
                }
                self.enabled.remove(input);
                self.altered.insert(input);
            }
        }
    }

    pub(super) fn refresh(&mut self, index: &Index, frame: usize) {
        #[cfg(feature = "measurement")]
        let _measurement =
            crate::measurement::profile::Scope::new(crate::measurement::profile::Phase::Refresh);
        let count = self.entry.count(frame);
        let dependency = || {
            index
                .affected
                .symbol(frame)
                .filter_map(|symbol| self.trigger.get(&symbol))
        };
        let selected: SmallVec<[(Key, usize); 4]> = if index.invalidated(frame) {
            self.entry
                .range(Key::frame(frame))
                .map(|(key, &position)| (key, position))
                .collect()
        } else if count <= 16
            || count <= self.empty.len() + dependency().map(Vec::len).sum::<usize>()
        {
            self.entry
                .range(Key::frame(frame))
                .filter(|(key, _)| {
                    let plan = self.catalog.input(key.input);
                    plan.empty()
                        || plan
                            .dependency()
                            .iter()
                            .any(|&value| index.affected.includes(frame, value))
                })
                .map(|(key, &position)| (key, position))
                .collect()
        } else {
            dependency()
                .flatten()
                .copied()
                .chain(&self.empty)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .flat_map(|input| self.entry.range(Key::input(frame, input)))
                .map(|(key, &position)| (key, position))
                .collect()
        };
        for (key, position) in selected {
            let entry = &mut self.store[position];
            self.storage -= entry.retained();
            let viable = entry.viable();
            if entry.advance(index, self.generation) {
                self.preparation += 1;
                if let Some(ready) = &mut self.ready
                    && entry.viable() != viable
                {
                    if viable {
                        ready.remove(&key);
                    } else {
                        ready.insert(key, position);
                    }
                }
            }
            self.storage += entry.retained();
        }
    }
}
