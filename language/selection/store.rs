use crate::budget::{Account, Reservation};
use crate::hashing::Builder;
use crate::particle::Match;
use crate::pattern::Pattern;
use crate::preparation::cache::{Cache, Request};
use crate::state::World;
use crate::term::Term;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

struct Entry {
    pattern: Arc<Pattern<Term>>,
    _reservation: Reservation,
}

pub(crate) struct Store {
    gate: OnceLock<Arc<crate::gate::Store>>,
    transcript: OnceLock<Arc<crate::search::Store>>,
    capacity: usize,
    visited: AtomicBool,
    storage: OnceLock<Box<Storage>>,
}

impl Store {
    pub fn new(capacity: usize) -> Self {
        Self {
            gate: OnceLock::new(),
            transcript: OnceLock::new(),
            capacity,
            visited: AtomicBool::new(false),
            storage: OnceLock::new(),
        }
    }

    pub fn subscribe(self: &Arc<Self>) -> Option<Arc<Self>> {
        self.visited
            .swap(true, Ordering::Relaxed)
            .then(|| self.clone())
    }

    pub fn transcript(&self) -> &Arc<crate::search::Store> {
        self.transcript
            .get_or_init(|| Arc::new(crate::search::Store::new(self.capacity)))
    }

    pub fn gate(&self) -> &Arc<crate::gate::Store> {
        self.gate
            .get_or_init(|| Arc::new(crate::gate::Store::new(self.capacity)))
    }

    pub fn compile(&self, pattern: &[Term]) -> Arc<Pattern<Term>> {
        self.storage
            .get_or_init(|| Box::new(Storage::new(self.capacity)))
            .compile(pattern)
    }

    pub fn select(&self, pattern: &Arc<Pattern<Term>>, world: &Arc<World>) -> Match {
        self.storage
            .get_or_init(|| Box::new(Storage::new(self.capacity)))
            .select(pattern, world)
    }

    pub fn evict(&self) {
        if let Some(transcript) = self.transcript.get() {
            transcript.evict();
        }
        if let Some(gate) = self.gate.get() {
            gate.evict();
        }
        if let Some(storage) = self.storage.get() {
            storage.evict();
        }
    }

    pub fn retained(&self) -> usize {
        self.transcript
            .get()
            .map_or(0, |transcript| transcript.retained())
            + self.gate.get().map_or(0, |gate| gate.retained())
            + self.storage.get().map_or(0, |storage| storage.retained())
    }
}

struct Storage {
    account: Account,
    preparation: Cache<Pattern<Term>>,
    fragment: Mutex<HashMap<Vec<Term>, Entry, Builder>>,
}

impl Storage {
    pub fn new(capacity: usize) -> Self {
        let account = Account::new(capacity);
        Self {
            preparation: Cache::new(account.clone()),
            account,
            fragment: Mutex::new(HashMap::default()),
        }
    }

    pub fn compile(&self, pattern: &[Term]) -> Arc<Pattern<Term>> {
        let value = pattern
            .iter()
            .map(|term| Term::new(term.value, term.capture))
            .collect::<Vec<_>>();
        if let Some(entry) = self.fragment.lock().unwrap().get(&value) {
            return entry.pattern.clone();
        }
        let pattern = Arc::new(Pattern::new(&value));
        let mut fragment = self.fragment.lock().unwrap();
        if let Some(entry) = fragment.get(&value) {
            return entry.pattern.clone();
        }
        let retained = value.len() * 2 + pattern.width + pattern.group.len() * 2 + 4;
        let reservation = self.account.reserve(retained).or_else(|| {
            fragment.retain(|_, entry| Arc::strong_count(&entry.pattern) > 1);
            self.account.reserve(retained)
        });
        if let Some(reservation) = reservation {
            fragment.insert(
                value,
                Entry {
                    pattern: pattern.clone(),
                    _reservation: reservation,
                },
            );
        }
        pattern
    }

    pub fn select(&self, pattern: &Arc<Pattern<Term>>, world: &Arc<World>) -> Match {
        self.preparation.select(
            Request {
                identity: pattern,
                world,
                context: 0,
            },
            || Match::compiled(pattern, &world.particle),
        )
    }

    pub fn evict(&self) {
        self.preparation.evict();
        self.fragment.lock().unwrap().clear();
    }

    pub fn retained(&self) -> usize {
        self.account.retained()
    }
}
