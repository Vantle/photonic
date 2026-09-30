use crate::catalog::Catalog;
use crate::dispatch::Network;
use crate::index::Index;
use crate::layout::Layout;
use crate::place::Place;
use crate::reader::Read;
use crate::slot::Slot;
use crate::state::State;
use std::sync::Arc;
use std::task::Poll;

pub(super) struct Match {
    pub rule: usize,
    pub frame: usize,
    pub owner: usize,
    pub read: Place,
    pub selection: Vec<Slot>,
}

// The matches of a configuration, one at a time, so a caller can stop before it has them all.
struct Scan {
    index: Index,
    network: Network,
}

impl Iterator for Scan {
    type Item = Match;

    fn next(&mut self) -> Option<Match> {
        loop {
            let delivery = match self.network.next(&self.index) {
                Poll::Ready(Some(delivery)) => delivery,
                Poll::Ready(None) => return None,
                Poll::Pending => continue,
            };
            if let Read::World(site, _) = delivery.read
                && !crate::slot::admits(&delivery.selection, self.index.world(site))
            {
                continue;
            }
            return Some(Match {
                rule: delivery.rule,
                frame: delivery.frame,
                owner: delivery.owner,
                read: delivery.read.place(&self.index),
                selection: delivery.selection,
            });
        }
    }
}

pub(super) fn scan(catalog: &Arc<Catalog>, state: &Arc<State>) -> impl Iterator<Item = Match> {
    let layout = Layout::new(state);
    let index = Index::prepared(state.clone(), layout.reach.frame);
    let network = Network::with(catalog.clone(), &index);
    Scan { index, network }
}
