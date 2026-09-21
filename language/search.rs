use crate::slot::Slot;
use crate::term::Term;
use cursor::Cursor;
use std::sync::Arc;
use std::task::Poll;

mod cursor;
mod key;
mod replay;
mod store;
mod transcript;

pub(crate) use store::Store;

pub struct Search {
    cursor: Cursor,
    replay: Option<Box<replay::Replay>>,
}

impl Search {
    pub(crate) fn cost(&self) -> usize {
        if matches!(self.replay.as_deref(), Some(replay::Replay::Playing { .. })) {
            return 0;
        }
        self.cursor.cost()
    }

    pub(crate) fn eligible(pattern: &[Vec<Term>]) -> bool {
        pattern.len() > 1 && pattern.iter().map(Vec::len).sum::<usize>() >= 32
    }

    pub fn new(pattern: Vec<Vec<Term>>, index: Arc<crate::index::Index>, frame: usize) -> Self {
        Self::prepared(
            Arc::new(crate::selection::Selection::new(
                Arc::new(pattern),
                &index,
                frame,
            )),
            index,
        )
    }

    pub(crate) fn prepared(
        selection: Arc<crate::selection::Selection>,
        index: Arc<crate::index::Index>,
    ) -> Self {
        Self {
            cursor: Cursor::prepared(selection, index),
            replay: None,
        }
    }

    pub(crate) fn shared(
        pattern: Vec<Vec<Term>>,
        index: Arc<crate::index::Index>,
        frame: usize,
        store: &Arc<crate::selection::Store>,
    ) -> Self {
        let eligible = Self::eligible(&pattern);
        let selection =
            crate::selection::Selection::shared(Arc::new(pattern), &index, frame, store);
        let mut search = Self::prepared(Arc::new(selection), index);
        if eligible {
            search.replay = Some(Box::new(replay::Replay::new(
                &search.cursor,
                store.transcript().clone(),
            )));
        }
        search
    }

    pub(crate) fn viable(&self) -> bool {
        self.cursor.viable()
    }

    pub(crate) fn resident(&self) -> usize {
        self.cursor.resident() + self.replay.as_deref().map_or(0, replay::Replay::retained)
    }

    pub(crate) fn retained(&self) -> usize {
        self.cursor.selection.retained() + self.resident()
    }

    pub(crate) fn evict(&mut self) {
        if let Some(mut replay) = self.replay.take() {
            replay.evict(&mut self.cursor);
        }
        self.cursor.evict();
    }

    #[inline]
    pub fn step(&mut self) -> Poll<Option<Vec<Slot>>> {
        match &mut self.replay {
            Some(replay) => replay.step(&mut self.cursor),
            None => self.cursor.step(),
        }
    }
}

#[cfg(test)]
#[path = "test/transcript.rs"]
mod test;
