use crate::program::Symbol;
use hashing::Builder;
use std::collections::HashSet;

#[derive(Default)]
pub(crate) struct Delta {
    pub toggled: HashSet<Symbol, Builder>,
    pub invalidated: Vec<usize>,
    pub repopulated: Vec<usize>,
    pub affected: crate::affected::Set,
    pub removal: Vec<usize>,
    pub insertion: Vec<usize>,
}

impl Delta {
    pub fn clear(&mut self) {
        self.toggled.clear();
        self.invalidated.clear();
        self.repopulated.clear();
        self.affected.clear();
        self.removal.clear();
        self.insertion.clear();
    }

    pub fn retained(&self) -> usize {
        self.toggled.len()
            + self.invalidated.len()
            + self.repopulated.len()
            + self.affected.retained()
            + self.removal.len()
            + self.insertion.len()
    }
}
