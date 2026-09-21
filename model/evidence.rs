use crate::context;
use crate::failure::Failure;
use crate::history::History;
use crate::occurrence;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evidence {
    pub(crate) read: BTreeMap<occurrence::Identity, occurrence::Occurrence>,
    pub(crate) context: BTreeSet<context::Identity>,
    pub(crate) history: History,
}

impl Evidence {
    pub fn read(&self) -> BTreeSet<occurrence::Identity> {
        self.read.keys().copied().collect()
    }

    pub fn witness(&self) -> impl Iterator<Item = &occurrence::Occurrence> {
        self.read.values()
    }

    pub fn context(&self) -> &BTreeSet<context::Identity> {
        &self.context
    }

    pub fn history(&self) -> &History {
        &self.history
    }

    pub(crate) fn append(&mut self, source: Self) -> Result<(), Failure> {
        self.history.permits(&source.history)?;
        for (&identity, value) in &source.read {
            if self
                .read
                .get(&identity)
                .is_some_and(|previous| previous != value)
            {
                return Err(Failure::Identity(identity));
            }
        }
        self.read.extend(source.read);
        self.context.extend(source.context);
        Ok(())
    }
}
