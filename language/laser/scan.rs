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
    pub read: Option<Place>,
    pub selection: Vec<Slot>,
}

pub(super) fn scan(catalog: &Arc<Catalog>, state: &Arc<State>) -> Vec<Match> {
    let layout = Layout::new(state);
    let index = Index::prepared(state.clone(), layout.reach.frame);
    let mut network = Network::with(catalog.clone(), &index);
    let mut found = Vec::new();
    loop {
        let delivery = match network.next(&index) {
            Poll::Ready(Some(delivery)) => delivery,
            Poll::Ready(None) => return found,
            Poll::Pending => continue,
        };
        if let Some(Read::World(site, _)) = delivery.read
            && !crate::slot::admits(&delivery.selection, index.world(site))
        {
            continue;
        }
        found.push(Match {
            rule: delivery.rule,
            frame: delivery.frame,
            owner: delivery.owner,
            read: delivery.read.map(|read| read.place(&index)),
            selection: delivery.selection,
        });
    }
}
