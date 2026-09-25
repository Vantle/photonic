use crate::budget::{Account, Reservation};
use std::sync::Arc;

const LIMIT: usize = 4096;

pub(crate) struct Selection {
    pub site: Vec<usize>,
    reservation: Option<Reservation>,
}

impl Selection {
    pub fn new(site: Vec<usize>, account: &Account) -> Arc<Self> {
        let retained = site.len() + 1;
        let reservation = if retained <= LIMIT {
            account.reserve(retained)
        } else {
            None
        };
        Arc::new(Self { site, reservation })
    }

    pub fn admitted(&self) -> bool {
        self.reservation.is_some()
    }
}
