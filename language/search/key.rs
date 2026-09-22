use super::Cursor;
use crate::state::World;
use crate::term::Term;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Weak};

struct Occurrence {
    position: usize,
    world: Weak<World>,
}

impl PartialEq for Occurrence {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position && self.world.ptr_eq(&other.world)
    }
}

impl Eq for Occurrence {}

impl Hash for Occurrence {
    fn hash<State: Hasher>(&self, state: &mut State) {
        self.position.hash(state);
        self.world.as_ptr().hash(state);
    }
}

#[derive(Eq, Hash, PartialEq)]
pub(super) struct Key {
    pattern: Arc<Vec<Vec<Term>>>,
    domain: Vec<Vec<Occurrence>>,
    order: Vec<usize>,
}

impl Key {
    pub fn new(cursor: &Cursor) -> Self {
        Self {
            pattern: cursor.selection.pattern.clone(),
            order: cursor.selection.order.clone(),
            domain: cursor
                .candidate
                .iter()
                .map(|domain| {
                    domain
                        .iter()
                        .map(|&position| Occurrence {
                            position: cursor.index.world(position),
                            world: Arc::downgrade(
                                &cursor.index.state.world[cursor.index.world(position)],
                            ),
                        })
                        .collect()
                })
                .collect(),
        }
    }

    pub fn alive(&self) -> bool {
        self.domain
            .iter()
            .flatten()
            .all(|occurrence| occurrence.world.strong_count() != 0)
    }

    pub fn retained(&self) -> usize {
        self.pattern.iter().map(Vec::len).sum::<usize>()
            + self.order.len()
            + self.domain.len()
            + self
                .domain
                .iter()
                .map(|domain| domain.len() * 2)
                .sum::<usize>()
            + 1
    }
}
