use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Budget {
    capacity: usize,
    retained: AtomicUsize,
}

impl Budget {
    #[inline]
    fn reserve(&self, size: usize) -> bool {
        self.retained
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |retained| {
                retained
                    .checked_add(size)
                    .filter(|&next| next <= self.capacity)
            })
            .is_ok()
    }

    #[inline]
    fn release(&self, size: usize) {
        self.retained.fetch_sub(size, Ordering::Relaxed);
    }
}

#[derive(Clone)]
pub(crate) struct Account {
    budget: Arc<Budget>,
    retained: Option<Arc<AtomicUsize>>,
}

impl Account {
    pub fn new(capacity: usize) -> Self {
        Self {
            budget: Arc::new(Budget {
                capacity,
                retained: AtomicUsize::new(0),
            }),
            retained: Some(Arc::new(AtomicUsize::new(0))),
        }
    }

    // Traces and match caches report their size to the traversal that holds them, so they draw on
    // the same budget without this account counting them a second time.
    pub fn share(&self) -> Self {
        Self {
            budget: self.budget.clone(),
            retained: None,
        }
    }

    #[inline]
    fn acquire(&self, size: usize) -> bool {
        if !self.budget.reserve(size) {
            return false;
        }
        if let Some(retained) = &self.retained {
            retained.fetch_add(size, Ordering::Relaxed);
        }
        true
    }

    #[inline]
    fn release(&self, size: usize) {
        if let Some(retained) = &self.retained {
            retained.fetch_sub(size, Ordering::Relaxed);
        }
        self.budget.release(size);
    }

    #[inline]
    pub fn reserve(&self, size: usize) -> Option<Reservation> {
        self.acquire(size).then(|| Reservation {
            account: self.clone(),
            size,
        })
    }

    #[inline]
    pub fn retained(&self) -> usize {
        self.retained
            .as_ref()
            .map_or(0, |retained| retained.load(Ordering::Relaxed))
    }
}

pub(crate) struct Reservation {
    account: Account,
    size: usize,
}

impl Reservation {
    #[inline]
    pub fn grow(&mut self, size: usize) -> bool {
        if !self.account.acquire(size) {
            return false;
        }
        self.size += size;
        true
    }

    #[inline]
    pub fn duplicate(&self) -> Option<Self> {
        self.account.reserve(self.size)
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        self.account.release(self.size);
    }
}
