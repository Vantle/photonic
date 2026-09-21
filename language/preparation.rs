pub(crate) mod cache;
mod key;

use crate::factor::Budget;
use crate::particle::Match;
use crate::pattern::Pattern;
use crate::program::Symbol;
use crate::state::World;
use cache::Cache;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

pub(crate) struct Request<'a> {
    pub pattern: &'a Arc<Pattern>,
    pub world: &'a Arc<World>,
    pub owner: usize,
}

pub(crate) struct Store(Cache<Pattern>);

impl Store {
    pub fn new(budget: Arc<Budget>, accounting: Arc<AtomicUsize>) -> Self {
        Self(Cache::new(budget, accounting))
    }

    pub fn select(&self, request: Request<'_>) -> Match {
        self.0.select(
            cache::Request {
                identity: request.pattern,
                world: request.world,
                context: if request
                    .pattern
                    .group
                    .iter()
                    .any(|group| matches!(group.value, Symbol::Rule(_)))
                {
                    request.owner
                } else {
                    0
                },
            },
            || {
                Match::prepared(
                    request.pattern,
                    Some(request.owner),
                    &request.world.particle,
                )
            },
        )
    }

    pub fn evict(&self) {
        self.0.evict();
    }
}

#[cfg(test)]
#[path = "test/preparation.rs"]
mod test;
