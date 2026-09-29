use crate::store::Store;
use crate::table::Table;
use crate::tally::Tally;
use crate::upload::Upload;
use crate::work::Work;
use photonic::laser::net::{Cycle, Net};
use photonic::runtime::Limit;

// One exploration under way: the net, its tables and their upload, the markings found so far, the
// memory its passes reuse, what the passes add up to, and the net's work allowance, the limits and
// the cycle choice it explores under.
pub struct Search<'net> {
    pub net: &'net mut Net,
    pub table: Table,
    pub upload: Upload,
    pub store: Store,
    pub work: Work,
    pub tally: Tally,
    pub allowance: usize,
    pub limit: Limit,
    pub cycle: Cycle,
}
