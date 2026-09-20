use crate::particle::Match;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::Poll;

pub(crate) struct Budget {
    capacity: usize,
    retained: AtomicUsize,
}

impl Budget {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            retained: AtomicUsize::new(0),
        }
    }

    fn reserve(&self, size: usize) -> bool {
        self.retained
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |retained| {
                retained
                    .checked_add(size)
                    .filter(|&next| next <= self.capacity)
            })
            .is_ok()
    }
}

enum Mode {
    Fresh,
    Visited,
    Repeated,
    Streaming,
    Recording(Box<Cache>),
}

struct Cache {
    budget: Arc<Budget>,
    binding: Vec<Vec<usize>>,
    cursor: usize,
    retained: usize,
    complete: bool,
}

impl Drop for Cache {
    fn drop(&mut self) {
        self.budget
            .retained
            .fetch_sub(self.retained, Ordering::Relaxed);
    }
}

enum Storage {
    Direct(Match),
    Retained(Box<Stream>),
}

pub(crate) struct Cursor(Storage);

impl Cursor {
    pub fn new(search: Match, budget: Option<Arc<Budget>>) -> Self {
        Self(match budget.filter(|_| search.viable()) {
            Some(budget) => Storage::Retained(Box::new(Stream::new(search, budget))),
            None => Storage::Direct(search),
        })
    }

    pub fn reset(&mut self) {
        match &mut self.0 {
            Storage::Direct(search) => search.reset(),
            Storage::Retained(stream) => stream.reset(),
        }
    }

    pub fn step(&mut self, allowance: usize) -> Poll<Option<Vec<usize>>> {
        match &mut self.0 {
            Storage::Direct(search) => search.step(),
            Storage::Retained(stream) => stream.step(allowance),
        }
    }

    pub fn cached(&self) -> usize {
        match &self.0 {
            Storage::Direct(_) => 0,
            Storage::Retained(stream) => stream.cached(),
        }
    }

    pub fn evict(&mut self) {
        if let Storage::Retained(stream) = &mut self.0 {
            stream.evict();
        }
    }

    pub fn retained(&self) -> usize {
        match &self.0 {
            Storage::Direct(search) => search.retained(),
            Storage::Retained(stream) => stream.retained(),
        }
    }
}

struct Stream {
    search: Match,
    budget: Arc<Budget>,
    mode: Mode,
}

impl Stream {
    fn new(search: Match, budget: Arc<Budget>) -> Self {
        Self {
            search,
            budget,
            mode: Mode::Fresh,
        }
    }

    fn reset(&mut self) {
        if let Mode::Recording(cache) = &mut self.mode {
            cache.cursor = 0;
            return;
        }
        self.search.reset();
        if matches!(self.mode, Mode::Visited) {
            self.mode = Mode::Repeated;
        }
    }

    fn step(&mut self, allowance: usize) -> Poll<Option<Vec<usize>>> {
        if matches!(self.mode, Mode::Fresh) {
            self.mode = Mode::Visited;
        }
        if matches!(self.mode, Mode::Repeated) {
            let budget = &self.budget;
            self.mode = if allowance > 0 && budget.reserve(1) {
                Mode::Recording(Box::new(Cache {
                    budget: budget.clone(),
                    binding: Vec::new(),
                    cursor: 0,
                    retained: 1,
                    complete: false,
                }))
            } else {
                Mode::Streaming
            };
            return self.advance(allowance.saturating_sub(1));
        }
        self.advance(allowance)
    }

    fn advance(&mut self, allowance: usize) -> Poll<Option<Vec<usize>>> {
        let Mode::Recording(cache) = &mut self.mode else {
            return self.search.step();
        };
        if let Some(binding) = cache.binding.get(cache.cursor) {
            cache.cursor += 1;
            return Poll::Ready(Some(binding.clone()));
        }
        if cache.complete {
            return Poll::Ready(None);
        }
        let result = self.search.step();
        match &result {
            Poll::Ready(Some(binding)) => {
                let size = binding.len() + 1;
                if size <= allowance && cache.budget.reserve(size) {
                    cache.retained += size;
                    cache.binding.push(binding.clone());
                    cache.cursor += 1;
                } else {
                    self.mode = Mode::Streaming;
                }
            }
            Poll::Ready(None) => cache.complete = true,
            Poll::Pending => self.mode = Mode::Streaming,
        }
        result
    }

    fn cached(&self) -> usize {
        match &self.mode {
            Mode::Recording(cache) => cache.retained,
            _ => 0,
        }
    }

    fn evict(&mut self) {
        let Mode::Recording(cache) = &self.mode else {
            return;
        };
        if cache.cursor < cache.binding.len() {
            self.search.reset();
            for _ in 0..cache.cursor {
                let _ = self.search.step();
            }
        }
        self.mode = Mode::Streaming;
    }

    fn retained(&self) -> usize {
        self.search.retained() + self.cached()
    }
}

#[cfg(test)]
#[path = "test/factor.rs"]
mod test;
