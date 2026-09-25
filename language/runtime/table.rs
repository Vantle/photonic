use crate::application::Owner;
use crate::hashing::Builder;
use crate::index::Index;
use crate::place::Place;
use crate::search::Search;
use crate::slot::Slot;
use crate::term::Term;
use indexmap::IndexSet;
use std::collections::HashMap;
use std::sync::Arc;
use std::task::Poll;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct Query {
    pub target: usize,
    pub frame: usize,
    pub pattern: Vec<Vec<Term>>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct Consumer {
    pub view: usize,
    pub frame: usize,
    pub owner: Owner<usize>,
    pub rule: usize,
    pub read: Option<Place>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Request {
    cache: usize,
    consumer: Consumer,
}

struct Cache {
    search: Option<Search>,
    binding: Vec<Arc<Vec<Slot>>>,
    listener: Vec<usize>,
    retained: usize,
}

#[derive(Default)]
pub(super) struct Schedule {
    pub search: Option<usize>,
    pub delivery: Vec<usize>,
}

pub(super) struct Delivery {
    pub consumer: Consumer,
    pub selection: Arc<Vec<Slot>>,
    pub again: bool,
}

#[derive(Default)]
pub(super) struct Table {
    query: HashMap<Query, usize, Builder>,
    cache: Vec<Cache>,
    request: IndexSet<Request, Builder>,
    cursor: Vec<usize>,
    active: Vec<bool>,
    binding: usize,
    retained: usize,
    preparation: Option<Arc<crate::selection::Store>>,
}

impl Table {
    pub fn subscribe(&mut self, key: Query, consumer: Consumer, index: Arc<Index>) -> Schedule {
        let mut schedule = Schedule::default();
        let cache = if let Some(&cache) = self.query.get(&key) {
            cache
        } else {
            let cache = self.cache.len();
            let search = if Search::eligible(&key.pattern)
                || crate::gate::Store::eligible(key.pattern.len())
                || key
                    .pattern
                    .iter()
                    .any(|particle| crate::particle::wide(particle.len()))
            {
                let store = self
                    .preparation
                    .get_or_insert_with(|| Arc::new(crate::selection::Store::new(65_536)));
                Search::shared(key.pattern.clone(), index, key.frame, store)
            } else {
                Search::new(key.pattern.clone(), index, key.frame)
            };
            let viable = search.viable();
            let retained = if viable { search.retained() } else { 0 };
            self.retained += retained;
            self.cache.push(Cache {
                search: viable.then_some(search),
                binding: Vec::new(),
                listener: Vec::new(),
                retained,
            });
            self.query.insert(key, cache);
            if viable {
                schedule.search = Some(cache);
            }
            cache
        };
        if self.cache[cache].search.is_none() && self.cache[cache].binding.is_empty() {
            return schedule;
        }
        let (index, fresh) = self.request.insert_full(Request { cache, consumer });
        if !fresh {
            return schedule;
        }
        self.cursor.push(0);
        self.active.push(false);
        self.cache[cache].listener.push(index);
        if !self.cache[cache].binding.is_empty()
            && !std::mem::replace(&mut self.active[index], true)
        {
            schedule.delivery.push(index);
        }
        schedule
    }

    pub fn take(&mut self, index: usize) -> Search {
        self.cache[index].search.take().unwrap()
    }

    pub fn advance(
        &mut self,
        index: usize,
        search: Search,
        progress: Poll<Option<Vec<Slot>>>,
    ) -> Schedule {
        let cache = &mut self.cache[index];
        let retained = search.retained();
        self.retained = self.retained - cache.retained + retained;
        cache.retained = retained;
        cache.search = Some(search);
        if matches!(progress, Poll::Ready(None)) {
            self.retained -= cache.retained;
            cache.retained = 0;
            cache.search = None;
            return Schedule::default();
        }
        let mut schedule = Schedule {
            search: Some(index),
            delivery: Vec::new(),
        };
        if let Poll::Ready(Some(binding)) = progress {
            self.binding += 1;
            cache.binding.push(Arc::new(binding));
            for &listener in &cache.listener {
                if !std::mem::replace(&mut self.active[listener], true) {
                    schedule.delivery.push(listener);
                }
            }
        }
        schedule
    }

    pub fn deliver(&mut self, index: usize) -> Option<Delivery> {
        self.active[index] = false;
        let request = &self.request[index];
        let cache = &self.cache[request.cache];
        let selection = cache.binding.get(self.cursor[index])?.clone();
        self.cursor[index] += 1;
        let again = self.cursor[index] < cache.binding.len()
            && !std::mem::replace(&mut self.active[index], true);
        Some(Delivery {
            consumer: request.consumer.clone(),
            selection,
            again,
        })
    }

    pub fn retained(&self) -> usize {
        self.request.len()
            + self.cache.len()
            + self.binding
            + self.retained
            + self
                .preparation
                .as_ref()
                .map_or(0, |store| store.retained())
    }

    pub fn evict(&mut self) {
        for cache in &mut self.cache {
            if let Some(search) = &mut cache.search {
                search.evict();
                let retained = search.retained();
                self.retained = self.retained - cache.retained + retained;
                cache.retained = retained;
            }
        }
        if let Some(store) = &self.preparation {
            store.evict();
        }
    }
}

#[cfg(test)]
#[path = "../test/table.rs"]
mod test;
