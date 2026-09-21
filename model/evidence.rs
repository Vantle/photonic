use crate::context;
use crate::history::History;
use crate::occurrence;
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evidence {
    pub(crate) read: BTreeSet<occurrence::Identity>,
    pub(crate) context: BTreeSet<context::Identity>,
    pub(crate) history: History,
}

impl Evidence {
    pub fn read(&self) -> &BTreeSet<occurrence::Identity> {
        &self.read
    }

    pub fn context(&self) -> &BTreeSet<context::Identity> {
        &self.context
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub(crate) fn append(&mut self, source: Self) {
        self.read.extend(source.read);
        self.context.extend(source.context);
    }
}
