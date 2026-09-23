use crate::status::Status;
use crate::support::{Atom, Clause, Support};
use indexmap::IndexSet;
use std::sync::OnceLock;

#[derive(Default)]
pub(crate) struct Store {
    clause: IndexSet<Clause, crate::hashing::Builder>,
    support: OnceLock<Support>,
}

impl Store {
    pub fn insert(&mut self, clause: Clause) {
        let (position, fresh) = self.clause.insert_full(clause);
        if fresh && let Some(support) = self.support.get_mut() {
            support.insert(&self.clause[position]);
        }
    }

    pub fn evaluate(&self) -> &Support {
        self.support.get_or_init(|| Support::new(&self.clause))
    }

    pub fn status(&self, atom: Atom) -> Status {
        self.evaluate().status(atom)
    }

    pub fn retained(&self) -> usize {
        self.clause.len()
    }
}
