use super::cursor::Cursor;
use super::space::Space;
use super::stream::Stream;
use crate::index::Index;
use crate::slot::Slot;
use std::sync::Arc;
use std::task::Poll;

pub(super) struct Product {
    prefix: Stream,
    suffix: Cursor,
    active: bool,
}

impl Product {
    pub fn new(width: usize, budget: Arc<crate::factor::Budget>) -> Self {
        Self {
            prefix: Stream::new(width - 1, budget),
            suffix: Cursor::new(1),
            active: false,
        }
    }

    pub fn reset(&mut self) {
        self.prefix.reset();
        self.suffix.reset();
        self.active = false;
    }

    pub fn step(
        &mut self,
        space: &mut Space,
        order: &[usize],
        index: &Index,
    ) -> Poll<Option<Vec<Slot>>> {
        let (prefix, suffix) = order.split_at(order.len() - 1);
        if !self.active {
            return match self.prefix.step(space, prefix, index) {
                Poll::Ready(Some(binding)) => {
                    self.suffix.seed(binding);
                    self.active = true;
                    Poll::Pending
                }
                result => result,
            };
        }
        match self.suffix.step(space, suffix, index) {
            Poll::Ready(None) => {
                self.suffix.reset();
                self.active = false;
                Poll::Pending
            }
            result => result,
        }
    }

    pub fn evict(&mut self) {
        self.prefix.evict();
    }

    #[cfg(test)]
    pub fn size(&self) -> usize {
        self.prefix.size() + self.suffix.size() + 1
    }

    #[cfg(test)]
    pub fn cached(&self) -> usize {
        self.prefix.cached()
    }

    pub fn retained(&self) -> usize {
        self.prefix.retained() + self.suffix.retained() + 1
    }
}
