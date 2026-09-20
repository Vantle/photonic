use super::Delivery;
use crate::index::Index;
use crate::replay::Search;
use crate::slot::Slot;
use smallvec::SmallVec;
use std::task::Poll;

pub(super) struct Consumer {
    pub rule: usize,
    pub owner: usize,
    pub read: Option<crate::reader::Read>,
}

pub(super) struct Entry {
    search: Search,
    consumer: SmallVec<[Consumer; 1]>,
    selection: Option<Vec<Slot>>,
    cursor: usize,
    generation: usize,
}

impl Entry {
    pub fn new(search: Search, consumer: SmallVec<[Consumer; 1]>, generation: usize) -> Self {
        Self {
            search,
            consumer,
            selection: None,
            cursor: 0,
            generation,
        }
    }

    pub fn replace(&mut self, consumer: SmallVec<[Consumer; 1]>) {
        self.consumer = consumer;
    }

    pub fn advance(&mut self, index: &Index, generation: usize) -> bool {
        if self.generation == generation {
            return false;
        }
        self.generation = generation;
        self.search.advance(index);
        self.selection = None;
        self.cursor = 0;
        true
    }

    pub fn viable(&self) -> bool {
        self.search.viable()
    }

    pub fn reset(&mut self, index: &Index) {
        self.selection = None;
        self.cursor = 0;
        self.search.reset(index);
    }

    pub fn next(&mut self, index: &Index) -> Poll<Option<Delivery>> {
        if self.selection.is_none() {
            match self.search.step(index) {
                Poll::Ready(Some(selection)) => {
                    self.selection = Some(selection);
                    self.cursor = 0;
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
        let consumer = &self.consumer[self.cursor];
        self.cursor += 1;
        let selection = if self.cursor == self.consumer.len() {
            self.selection.take().unwrap()
        } else {
            self.selection.as_ref().unwrap().clone()
        };
        Poll::Ready(Some(Delivery {
            rule: consumer.rule,
            frame: self.search.frame(),
            owner: consumer.owner,
            read: consumer.read,
            selection,
        }))
    }

    pub fn evict(&mut self) {
        self.search.evict();
    }

    pub fn retained(&self) -> usize {
        self.search.retained()
            + self.consumer.len()
            + self.selection.as_ref().map_or(0, |selection| {
                selection
                    .iter()
                    .map(|slot| slot.token.len() + 1)
                    .sum::<usize>()
            })
            + 1
    }
}
