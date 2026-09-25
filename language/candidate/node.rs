use super::key::Key;
use super::selection::Selection;
use crate::budget::Account;
use crate::budget::Reservation;
use crate::index::Index;
use crate::revision::Revision;
use std::sync::{Arc, Mutex};

struct Version {
    revision: Revision,
    affected: bool,
    selection: Option<Arc<Selection>>,
    insertion: Option<Arc<Selection>>,
}

pub(crate) struct Node {
    key: Arc<Key>,
    account: Account,
    version: Mutex<Option<Version>>,
    _reservation: Reservation,
}

impl Node {
    pub(super) fn new(key: Arc<Key>, account: Account, reservation: Reservation) -> Self {
        Self {
            key,
            account,
            version: Mutex::new(None),
            _reservation: reservation,
        }
    }

    fn version<'version>(
        &self,
        version: &'version mut Option<Version>,
        index: &Index,
    ) -> &'version mut Version {
        if version
            .as_ref()
            .is_none_or(|version| version.revision != *index.revision())
        {
            *version = Some(self.advance(version.take(), index));
        }
        version.as_mut().unwrap()
    }

    fn advance(&self, previous: Option<Version>, index: &Index) -> Version {
        let mut version = Version {
            revision: index.revision().clone(),
            affected: self.key.affected(index),
            selection: None,
            insertion: None,
        };
        let Some(previous) =
            previous.filter(|previous| index.previous() == Some(&previous.revision))
        else {
            return version;
        };
        let Some(selection) = previous.selection else {
            return version;
        };
        if !version.affected {
            version.selection = Some(selection);
            return version;
        }
        let insertion = Selection::new(self.key.insertion(index), &self.account);
        if insertion.site.is_empty() && !selection.site.iter().any(|&site| index.removed(site)) {
            version.selection = Some(selection);
        } else {
            let mut site = selection
                .site
                .iter()
                .copied()
                .filter(|&site| !index.removed(site))
                .collect::<Vec<_>>();
            site.extend(&insertion.site);
            drop(selection);
            let selection = Selection::new(site, &self.account);
            if selection.admitted() {
                version.selection = Some(selection);
            }
        }
        if insertion.admitted() {
            version.insertion = Some(insertion);
        }
        version
    }

    pub(super) fn select(&self, index: &Index) -> Arc<Selection> {
        let mut version = self.version.lock().unwrap();
        let version = self.version(&mut version, index);
        if let Some(selection) = &version.selection {
            return selection.clone();
        }
        let selection = Selection::new(self.key.select(index), &self.account);
        if selection.admitted() {
            version.selection = Some(selection.clone());
        }
        selection
    }

    pub(super) fn publish(&self, index: &Index, selection: &Arc<Selection>) {
        if selection.admitted() {
            let mut version = self.version.lock().unwrap();
            self.version(&mut version, index).selection = Some(selection.clone());
        }
    }

    pub fn change(&self, index: &Index) -> super::Change {
        let mut version = self.version.lock().unwrap();
        let version = self.version(&mut version, index);
        let insertion = if let Some(selection) = &version.insertion {
            selection.clone()
        } else {
            let site = if version.affected {
                self.key.insertion(index)
            } else {
                Vec::new()
            };
            let selection = Selection::new(site, &self.account);
            if selection.admitted() {
                version.insertion = Some(selection.clone());
            }
            selection
        };
        super::Change {
            affected: version.affected,
            insertion,
        }
    }
}
