use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) struct Budget {
    capacity: usize,
    retained: AtomicUsize,
}

impl Budget {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            retained: AtomicUsize::new(0),
        }
    }

    #[inline]
    pub fn reserve(&self, size: usize) -> bool {
        self.retained
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |retained| {
                retained
                    .checked_add(size)
                    .filter(|&next| next <= self.capacity)
            })
            .is_ok()
    }

    #[inline]
    pub fn release(&self, size: usize) {
        self.retained.fetch_sub(size, Ordering::Relaxed);
    }

    #[cfg(test)]
    pub fn retained(&self) -> usize {
        self.retained.load(Ordering::Relaxed)
    }
}

#[derive(Clone)]
pub(crate) struct Account {
    budget: Arc<Budget>,
    retained: Arc<AtomicUsize>,
}

impl Account {
    pub fn new(capacity: usize) -> Self {
        Self {
            budget: Arc::new(Budget::new(capacity)),
            retained: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[inline]
    pub fn budget(&self) -> &Arc<Budget> {
        &self.budget
    }

    #[inline]
    pub fn reserve(&self, size: usize) -> Option<Reservation> {
        if !self.budget.reserve(size) {
            return None;
        }
        self.retained.fetch_add(size, Ordering::Relaxed);
        Some(Reservation {
            account: self.clone(),
            size,
        })
    }

    #[inline]
    pub fn retained(&self) -> usize {
        self.retained.load(Ordering::Relaxed)
    }
}

pub(crate) struct Reservation {
    account: Account,
    size: usize,
}

impl Drop for Reservation {
    fn drop(&mut self) {
        self.account
            .retained
            .fetch_sub(self.size, Ordering::Relaxed);
        self.account.budget.release(self.size);
    }
}
