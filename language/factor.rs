use crate::budget::{Account, Reservation};
use crate::particle::Match;

enum Mode {
    Fresh,
    Visited,
    Repeated,
    Streaming,
    Recording(Box<Cache>),
}

struct Cache {
    reservation: Reservation,
    binding: Vec<Vec<usize>>,
    cursor: usize,
    complete: bool,
}

enum Storage {
    Direct(Match),
    Retained(Box<Stream>),
}

pub(crate) struct Cursor(Storage);

impl Cursor {
    pub fn new(search: Match, budget: Option<Account>) -> Self {
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

    pub fn step(&mut self, allowance: usize) -> Option<Vec<usize>> {
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
    budget: Account,
    mode: Mode,
}

impl Stream {
    fn new(search: Match, budget: Account) -> Self {
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

    fn step(&mut self, allowance: usize) -> Option<Vec<usize>> {
        if matches!(self.mode, Mode::Fresh) {
            self.mode = Mode::Visited;
        }
        if matches!(self.mode, Mode::Repeated) {
            let reservation = if allowance > 0 {
                self.budget.reserve(1)
            } else {
                None
            };
            self.mode = reservation.map_or(Mode::Streaming, |reservation| {
                Mode::Recording(Box::new(Cache {
                    reservation,
                    binding: Vec::new(),
                    cursor: 0,
                    complete: false,
                }))
            });
            return self.advance(allowance.saturating_sub(1));
        }
        self.advance(allowance)
    }

    fn advance(&mut self, allowance: usize) -> Option<Vec<usize>> {
        let Mode::Recording(cache) = &mut self.mode else {
            return self.search.step();
        };
        if let Some(binding) = cache.binding.get(cache.cursor) {
            cache.cursor += 1;
            return Some(binding.clone());
        }
        if cache.complete {
            return None;
        }
        let result = self.search.step();
        let Some(binding) = &result else {
            cache.complete = true;
            return result;
        };
        let size = binding.len() + 1;
        if size <= allowance && cache.reservation.grow(size) {
            cache.binding.push(binding.clone());
            cache.cursor += 1;
        } else {
            self.mode = Mode::Streaming;
        }
        result
    }

    fn cached(&self) -> usize {
        match &self.mode {
            Mode::Recording(cache) => cache.reservation.size(),
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
