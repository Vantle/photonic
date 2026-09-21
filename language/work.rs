use crate::canonical;
use crate::search;
use crate::slot::Slot;
use std::task::Poll;

pub(crate) enum Work {
    Search(usize, search::Search),
    Normalize(usize, canonical::Search),
}

pub(crate) enum Result {
    Search(usize, search::Search, Poll<Option<Vec<Slot>>>),
    Normalize(usize, canonical::Search, bool),
}

impl Work {
    pub(crate) fn parallel(batch: &[Self]) -> bool {
        let mut cost = 0usize;
        let mut count = 0;
        for work in batch {
            if let Self::Normalize(_, search) = work
                && search.cost() >= 128
            {
                cost = cost.saturating_add(search.cost());
                count += 1;
            }
        }
        count >= 2 && cost >= 8192
    }

    pub(crate) fn advance(self) -> Result {
        match self {
            Self::Search(index, mut search) => {
                let progress = search.step();
                Result::Search(index, search, progress)
            }
            Self::Normalize(index, mut search) => {
                let complete = search.step();
                Result::Normalize(index, search, complete)
            }
        }
    }
}
