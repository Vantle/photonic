use super::Cursor;
use super::key::Key;
use super::store::{Entry, Store};
use super::transcript::{Playback, Transcript};
use crate::slot::Slot;
use std::sync::Arc;
use std::task::Poll;

pub(super) enum Replay {
    Recording {
        store: Arc<Store>,
        key: Key,
        transcript: Transcript,
    },
    Playing {
        entry: Arc<Entry>,
        playback: Playback,
    },
    Disabled,
}

impl Replay {
    pub fn new(cursor: &Cursor, store: Arc<Store>) -> Self {
        if store.capacity() == 0 {
            return Self::Disabled;
        }
        let key = Key::new(cursor);
        if key.retained() > 4096 {
            return Self::Disabled;
        }
        if let Some(entry) = store.find(&key) {
            return Self::Playing {
                entry,
                playback: Playback::default(),
            };
        }
        Self::Recording {
            store,
            key,
            transcript: Transcript::default(),
        }
    }

    pub fn step(&mut self, cursor: &mut Cursor) -> Poll<Option<Vec<Slot>>> {
        if let Self::Playing { entry, playback } = self {
            return playback.step(&entry.transcript);
        }
        let result = cursor.step();
        let Self::Recording { transcript, .. } = self else {
            return result;
        };
        if !transcript.append(&result) {
            *self = Self::Disabled;
            return result;
        }
        if matches!(result, Poll::Ready(None)) {
            let Self::Recording {
                store,
                key,
                transcript,
            } = std::mem::replace(self, Self::Disabled)
            else {
                unreachable!()
            };
            store.insert(key, transcript);
        }
        result
    }

    pub fn evict(&mut self, cursor: &mut Cursor) {
        if let Self::Playing { playback, .. } = self {
            for _ in 0..playback.progress {
                let _ = cursor.step();
            }
        }
        *self = Self::Disabled;
    }

    pub fn retained(&self) -> usize {
        match self {
            Self::Recording {
                key, transcript, ..
            } => key.retained() + transcript.retained,
            _ => 0,
        }
    }
}
