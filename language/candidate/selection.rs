use crate::factor::Budget;
use crate::reservation::Reservation;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

pub(crate) struct Selection {
    pub site: Vec<usize>,
    reservation: Option<Reservation>,
}

impl Selection {
    pub fn new(site: Vec<usize>, budget: &Arc<Budget>, accounting: &Arc<AtomicUsize>) -> Arc<Self> {
        let retained = site.len() + 1;
        let reservation = if retained <= 4096 {
            Reservation::new(budget, accounting, retained)
        } else {
            None
        };
        Arc::new(Self { site, reservation })
    }

    pub fn admitted(&self) -> bool {
        self.reservation.is_some()
    }
}
