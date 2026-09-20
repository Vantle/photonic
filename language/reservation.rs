use crate::factor::Budget;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) struct Reservation {
    budget: Arc<Budget>,
    accounting: Arc<AtomicUsize>,
    retained: usize,
}

impl Reservation {
    pub fn new(
        budget: &Arc<Budget>,
        accounting: &Arc<AtomicUsize>,
        retained: usize,
    ) -> Option<Self> {
        if !budget.reserve(retained) {
            return None;
        }
        accounting.fetch_add(retained, Ordering::Relaxed);
        Some(Self {
            budget: budget.clone(),
            accounting: accounting.clone(),
            retained,
        })
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        self.accounting.fetch_sub(self.retained, Ordering::Relaxed);
        self.budget.release(self.retained);
    }
}
