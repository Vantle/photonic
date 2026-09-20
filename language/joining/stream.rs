use super::cursor::Cursor;
use super::space::Space;
use crate::index::Index;
use crate::slot::Slot;
use std::sync::Arc;
use std::task::Poll;

enum Record {
    Waiting(usize),
    Binding(Vec<Slot>),
}

enum Mode {
    Fresh,
    Visited,
    Repeated,
    Streaming,
    Recording(Box<Cache>),
}

struct Cache {
    budget: Arc<crate::factor::Budget>,
    record: Vec<Record>,
    cursor: usize,
    offset: usize,
    progress: usize,
    retained: usize,
    complete: bool,
}

impl Cache {
    fn reserve(&mut self, size: usize) -> bool {
        if size > 4096 - self.retained || !self.budget.reserve(size) {
            return false;
        }
        self.retained += size;
        true
    }

    fn replay(&mut self) -> Option<Poll<Option<Vec<Slot>>>> {
        let record = self.record.get(self.cursor)?;
        self.progress += 1;
        Some(match record {
            Record::Waiting(count) => {
                self.offset += 1;
                if self.offset == *count {
                    self.cursor += 1;
                    self.offset = 0;
                }
                Poll::Pending
            }
            Record::Binding(binding) => {
                self.cursor += 1;
                Poll::Ready(Some(binding.clone()))
            }
        })
    }

    fn append(&mut self, result: &Poll<Option<Vec<Slot>>>) -> bool {
        if self.progress == 65536 {
            return false;
        }
        match result {
            Poll::Ready(None) => self.complete = true,
            Poll::Pending => {
                if let Some(Record::Waiting(count)) = self.record.last_mut() {
                    *count += 1;
                } else {
                    if !self.reserve(1) {
                        return false;
                    }
                    self.record.push(Record::Waiting(1));
                }
            }
            Poll::Ready(Some(binding)) => {
                let size = 1 + binding
                    .iter()
                    .map(|slot| slot.token.len() + 1)
                    .sum::<usize>();
                if !self.reserve(size) {
                    return false;
                }
                self.record.push(Record::Binding(binding.clone()));
            }
        }
        self.cursor = self.record.len();
        self.progress += 1;
        true
    }
}

impl Drop for Cache {
    fn drop(&mut self) {
        self.budget.release(self.retained);
    }
}

pub(super) struct Stream {
    source: Cursor,
    budget: Arc<crate::factor::Budget>,
    mode: Mode,
    restoration: Option<usize>,
}

impl Stream {
    pub fn new(width: usize, budget: Arc<crate::factor::Budget>) -> Self {
        Self {
            source: Cursor::new(width),
            budget,
            mode: Mode::Fresh,
            restoration: None,
        }
    }

    pub fn reset(&mut self) {
        if let Mode::Recording(cache) = &mut self.mode {
            cache.cursor = 0;
            cache.offset = 0;
            cache.progress = 0;
            return;
        }
        self.restoration = None;
        self.source.reset();
        if matches!(self.mode, Mode::Visited) {
            self.mode = Mode::Repeated;
        }
    }

    pub fn step(
        &mut self,
        space: &mut Space,
        order: &[usize],
        index: &Index,
    ) -> Poll<Option<Vec<Slot>>> {
        if let Some(progress) = self.restoration.take() {
            for _ in 0..progress {
                let _ = self.source.step(space, order, index);
            }
        }
        if matches!(self.mode, Mode::Fresh) {
            self.mode = Mode::Visited;
        }
        if matches!(self.mode, Mode::Repeated) {
            self.mode = if self.budget.reserve(1) {
                Mode::Recording(Box::new(Cache {
                    budget: self.budget.clone(),
                    record: Vec::new(),
                    cursor: 0,
                    offset: 0,
                    progress: 0,
                    retained: 1,
                    complete: false,
                }))
            } else {
                Mode::Streaming
            };
        }
        let Mode::Recording(cache) = &mut self.mode else {
            return self.source.step(space, order, index);
        };
        if let Some(result) = cache.replay() {
            return result;
        }
        if cache.complete {
            return Poll::Ready(None);
        }
        let result = self.source.step(space, order, index);
        if !cache.append(&result) {
            self.mode = Mode::Streaming;
        }
        result
    }

    pub fn evict(&mut self) {
        let Mode::Recording(cache) = &self.mode else {
            return;
        };
        self.restoration = Some(cache.progress);
        self.source.reset();
        self.mode = Mode::Streaming;
    }

    #[cfg(test)]
    pub fn size(&self) -> usize {
        self.source.size()
            + 1
            + match &self.mode {
                Mode::Recording(cache) => {
                    1 + cache
                        .record
                        .iter()
                        .map(|record| match record {
                            Record::Waiting(_) => 1,
                            Record::Binding(binding) => {
                                1 + binding
                                    .iter()
                                    .map(|slot| slot.token.len() + 1)
                                    .sum::<usize>()
                            }
                        })
                        .sum::<usize>()
                }
                _ => 0,
            }
    }

    #[cfg(test)]
    pub fn cached(&self) -> usize {
        match &self.mode {
            Mode::Recording(cache) => cache.retained,
            _ => 0,
        }
    }

    pub fn retained(&self) -> usize {
        self.source.retained()
            + match &self.mode {
                Mode::Recording(cache) => cache.retained,
                _ => 0,
            }
            + 1
    }
}
