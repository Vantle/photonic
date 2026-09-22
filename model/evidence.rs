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
    pub(crate) proof: Option<std::sync::Arc<crate::proof::Proof>>,
    pub(crate) qualified: BTreeMap<crate::support::Request, occurrence::Occurrence>,
    pub(crate) capture: BTreeSet<crate::capture::Capture>,
}

impl Evidence {
    pub fn proof(&self) -> Option<&crate::proof::Proof> {
        self.proof.as_deref()
    }

    pub fn qualified(
        &self,
    ) -> impl Iterator<Item = (&crate::support::Request, &occurrence::Occurrence)> {
        self.qualified.iter()
    }

    pub fn capture(&self) -> &BTreeSet<crate::capture::Capture> {
        &self.capture
    }

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
        if self.proof.is_some() && source.proof.is_some() && self.proof != source.proof {
            return Err(Failure::Source);
        }
        for (request, value) in &source.qualified {
            if self
                .qualified
                .get(request)
                .is_some_and(|previous| previous != value)
            {
                return Err(Failure::Identity(value.identity));
            }
        }
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
        self.proof = self.proof.take().or(source.proof);
        self.qualified.extend(source.qualified);
        self.capture.extend(source.capture);
        Ok(())
    }
}
