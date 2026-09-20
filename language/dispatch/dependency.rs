use super::{Key, Network};
use crate::index::Index;
use crate::program::Symbol;
use smallvec::SmallVec;
use std::collections::BTreeSet;

impl Network {
    pub(super) fn symbol(&mut self, symbol: Symbol, present: bool) {
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
                if self.missing[input] != 1 {
                    continue;
                }
                self.enabled.remove(&input);
                self.altered.insert(input);
                for &owner in self.catalog.owner(input) {
                    if self.scope[owner].remove(&input) {
                        self.retained -= 1;
                    }
                }
            }
        }
    }

    pub(super) fn refresh(&mut self, index: &Index, frame: usize) {
        let count = self.count.get(frame).copied().unwrap_or(0);
        let dependency = || {
            index
                .affected
                .get(&frame)
                .into_iter()
                .flatten()
                .filter_map(|symbol| self.trigger.get(symbol))
        };
        let selected: SmallVec<[(Key, usize); 4]> = if count <= 16
            || count <= self.empty.len() + dependency().map(Vec::len).sum::<usize>()
        {
            self.entry
                .range(Key::frame(frame))
                .filter(|(key, _)| {
                    let plan = self.catalog.input(key.input);
                    plan.empty()
                        || index.affected.get(&frame).is_some_and(|symbol| {
                            plan.dependency().iter().any(|value| symbol.contains(value))
                        })
                })
                .map(|(&key, &position)| (key, position))
                .collect()
        } else {
            dependency()
                .flatten()
                .chain(&self.empty)
                .copied()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .flat_map(|input| self.entry.range(Key::input(frame, input)))
                .map(|(&key, &position)| (key, position))
                .collect()
        };
        for (key, position) in selected {
            let entry = &mut self.store[position];
            self.storage -= entry.retained();
            if entry.advance(index, self.generation) {
                self.preparation += 1;
                if let Some(ready) = &mut self.ready {
                    if entry.viable() {
                        ready.insert(key, position);
                    } else {
                        ready.remove(&key);
                    }
                }
            }
            self.storage += entry.retained();
        }
    }
}
