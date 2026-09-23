use super::key::Key;
use crate::budget::{Account, Reservation};
use crate::hashing::Builder;
use crate::particle::{Match, Preparation};
use crate::state::World;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub(crate) struct Request<'a, Value> {
    pub identity: &'a Arc<Value>,
    pub world: &'a Arc<World>,
    pub context: usize,
}

struct Entry {
    preparation: Arc<Preparation>,
    _reservation: Reservation,
}

pub(crate) struct Cache<Value> {
    account: Account,
    entry: Mutex<HashMap<Key<Value>, Entry, Builder>>,
}

impl<Value> Cache<Value> {
    pub fn new(account: Account) -> Self {
        Self {
            account,
            entry: Mutex::new(HashMap::default()),
        }
    }

    #[inline]
    pub fn select(&self, request: Request<'_, Value>, prepare: impl FnOnce() -> Match) -> Match {
        let key = Key::new(&request);
        if let Some(entry) = self.entry.lock().unwrap().get(&key) {
            return Match::from(entry.preparation.clone());
        }
        let search = prepare();
        let mut entry = self.entry.lock().unwrap();
        if let Some(entry) = entry.get(&key) {
            return Match::from(entry.preparation.clone());
        }
        let preparation = search.preparation();
        let retained = preparation.retained() + 4;
        let reservation = self.account.reserve(retained).or_else(|| {
            entry.retain(|key, _| key.alive());
            self.account.reserve(retained)
        });
        if let Some(reservation) = reservation {
            entry.insert(
                key,
                Entry {
                    preparation,
                    _reservation: reservation,
                },
            );
        }
        search
    }

    pub fn evict(&self) {
        self.entry.lock().unwrap().clear();
    }
}
