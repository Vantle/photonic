use crate::canonical;
use crate::matching::Slot;
use crate::search;
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
