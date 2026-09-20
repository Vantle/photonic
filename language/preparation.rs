mod key;

use crate::factor::Budget;
use crate::particle::{Match, Preparation};
use crate::pattern::Pattern;
use crate::reservation::Reservation;
use crate::state::World;
use key::Key;
use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Mutex};

pub(crate) struct Request<'a> {
    pub pattern: &'a Arc<Pattern>,
    pub world: &'a Arc<World>,
    pub owner: usize,
}

struct Entry {
    preparation: Arc<Preparation>,
    _reservation: Reservation,
}

pub(crate) struct Store {
    budget: Arc<Budget>,
    accounting: Arc<AtomicUsize>,
    entry: Mutex<HashMap<Key, Entry>>,
}

impl Store {
    pub fn new(budget: Arc<Budget>, accounting: Arc<AtomicUsize>) -> Self {
        Self {
            budget,
            accounting,
            entry: Mutex::new(HashMap::new()),
        }
    }

    pub fn select(&self, request: Request<'_>) -> Match {
        let key = Key::new(&request);
        let mut entry = self.entry.lock().unwrap();
        if let Some(entry) = entry.get(&key) {
            return Match::from(entry.preparation.clone());
        }
        let search = Match::prepared(
            request.pattern,
            Some(request.owner),
            &request.world.particle,
        );
        let preparation = search.preparation();
        let retained = preparation.retained() + 4;
        let reservation =
            Reservation::new(&self.budget, &self.accounting, retained).or_else(|| {
                entry.retain(|key, _| key.alive());
                Reservation::new(&self.budget, &self.accounting, retained)
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

#[cfg(test)]
#[path = "test/preparation.rs"]
mod test;
