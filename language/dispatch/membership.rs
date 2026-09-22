use super::entry::Consumer;
use crate::catalog::Catalog;
use crate::hashing::Builder;
use crate::program::Symbol;
use crate::reader::Read;
use crate::state::Token;
use smallvec::SmallVec;
use std::collections::HashMap;

fn consumer(frame: usize, token: &Token) -> Option<Consumer> {
    let Symbol::Rule(rule) = token.value else {
        return None;
    };
    Some(Consumer {
        rule,
        owner: token.capture.unwrap(),
        read: Some(Read::Context(frame, token.id)),
    })
}

struct Population {
    value: crate::population::Set,
    group: HashMap<usize, SmallVec<[Consumer; 1]>, Builder>,
    retained: usize,
}

impl Population {
    fn new(frame: usize, value: &crate::population::Set, catalog: &Catalog) -> Self {
        let mut population = Self {
            value: value.clone(),
            group: HashMap::default(),
            retained: 1,
        };
        for consumer in value.iter().filter_map(|token| consumer(frame, token)) {
            population.insert(consumer, catalog);
        }
        population
    }

    fn insert(&mut self, consumer: Consumer, catalog: &Catalog) {
        let group = self.group.entry(catalog.rule(consumer.rule)).or_default();
        self.retained += 1 + usize::from(group.is_empty());
        group.push(consumer);
    }

    fn remove(&mut self, consumer: Consumer, catalog: &Catalog) {
        let input = catalog.rule(consumer.rule);
        let group = self.group.get_mut(&input).unwrap();
        let position = group.iter().position(|value| *value == consumer).unwrap();
        group.remove(position);
        self.retained -= 1;
        if group.is_empty() {
            self.group.remove(&input);
            self.retained -= 1;
        }
    }

    fn advance(&mut self, frame: usize, value: &crate::population::Set, catalog: &Catalog) {
        if self.value != *value {
            for position in self.value.removed(value) {
                if let Some(consumer) = consumer(frame, self.value.at(position)) {
                    self.remove(consumer, catalog);
                }
            }
            for position in value.removed(&self.value) {
                if let Some(consumer) = consumer(frame, value.at(position)) {
                    self.insert(consumer, catalog);
                }
            }
        }
        self.value = value.clone();
    }
}

pub(super) struct Index {
    frame: Option<Vec<Option<Population>>>,
    retained: usize,
}

impl Index {
    pub fn new(index: &crate::index::Index) -> Self {
        let frame = std::iter::repeat_with(|| None)
            .take(index.state.frame.len())
            .collect::<Vec<_>>();
        Self {
            frame: Some(frame),
            retained: 0,
        }
    }

    pub fn ensure(&mut self, index: &crate::index::Index, catalog: &Catalog, position: usize) {
        let Some(frame) = &mut self.frame else {
            return;
        };
        if frame[position].is_some() || !index.present(position) {
            return;
        }
        let population = Population::new(position, &index.state.frame[position].particle, catalog);
        self.retained += population.retained;
        frame[position] = Some(population);
    }

    pub fn advance(&mut self, index: &crate::index::Index, catalog: &Catalog) {
        let Some(frame) = &mut self.frame else {
            return;
        };
        frame.resize_with(frame.len().max(index.state.frame.len()), || None);
        for &position in &index.ownership {
            if !index.present(position) {
                if let Some(population) = frame[position].take() {
                    self.retained -= population.retained;
                }
                continue;
            }
            let value = &index.state.frame[position].particle;
            if let Some(population) = &mut frame[position] {
                self.retained -= population.retained;
                population.advance(position, value, catalog);
                self.retained += population.retained;
            }
        }
        frame.truncate(index.state.frame.len());
    }

    pub fn select<'scope>(
        &'scope self,
        catalog: &'scope Catalog,
        index: &'scope crate::index::Index,
        frame: usize,
        input: usize,
    ) -> impl Iterator<Item = Consumer> + 'scope {
        let cached = self
            .frame
            .as_ref()
            .and_then(|value| value.get(frame))
            .and_then(Option::as_ref)
            .map(|population| {
                population
                    .group
                    .get(&input)
                    .map_or(&[][..], |group| group.as_slice())
            });
        cached.into_iter().flatten().copied().chain(
            cached
                .is_none()
                .then(|| catalog.member(input))
                .into_iter()
                .flatten()
                .flat_map(move |&rule| index.local(frame, Symbol::Rule(rule)))
                .filter_map(move |token| consumer(frame, token)),
        )
    }

    pub fn evict(&mut self) -> usize {
        let released = self.retained();
        self.frame = None;
        self.retained = 0;
        released
    }

    pub fn retained(&self) -> usize {
        self.retained + self.frame.as_ref().map_or(0, Vec::len)
    }
}

#[cfg(test)]
#[path = "../test/consumer.rs"]
mod test;
