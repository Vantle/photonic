use crate::index::Index;
use crate::joining::Join;
use crate::slot::Slot;
#[cfg(test)]
use crate::term::Term;
#[cfg(test)]
use std::sync::Arc;
use std::task::Poll;

enum Step {
    Waiting(usize),
    Binding(Vec<Selection>),
}

struct Selection {
    site: usize,
    position: usize,
    token: Vec<usize>,
}

enum Mode {
    Dormant,
    Visited,
    Streaming,
    Recording(Box<Cache>),
}

struct Cache {
    record: Vec<Step>,
    cursor: usize,
    offset: usize,
    retained: usize,
    complete: bool,
    saturated: bool,
}

pub(crate) struct Search {
    join: Join,
    mode: Mode,
}

impl Search {
    pub fn waiting(&self) -> usize {
        match &self.mode {
            Mode::Dormant => 0,
            Mode::Recording(cache) => cache.waiting(),
            _ => self.join.waiting(),
        }
    }

    pub fn skip(&mut self, maximum: usize) -> usize {
        match &mut self.mode {
            Mode::Dormant => 0,
            Mode::Recording(cache) => cache.skip(maximum),
            _ => self.join.skip(maximum),
        }
    }

    #[cfg(test)]
    pub fn new(pattern: Arc<Vec<Vec<Term>>>, index: &Index, frame: usize) -> Self {
        Self {
            join: Join::new(pattern, index, frame),
            mode: Mode::Dormant,
        }
    }

    pub fn planned(request: crate::joining::Request<'_>) -> Self {
        #[cfg(feature = "measurement")]
        let _measurement = crate::measurement::profile::Scope::new(
            crate::measurement::profile::Phase::Preparation,
        );
        Self {
            join: Join::planned(request),
            mode: Mode::Dormant,
        }
    }

    pub fn advance(&mut self, index: &Index) {
        if self.join.update(index) {
            self.mode = Mode::Dormant;
        } else {
            self.reset(index);
        }
    }

    pub fn reset(&mut self, index: &Index) {
        if let Mode::Recording(cache) = &mut self.mode {
            cache.cursor = 0;
            cache.offset = 0;
            return;
        }
        self.join.reset(index);
        if matches!(self.mode, Mode::Visited) {
            self.mode = Mode::Recording(Box::new(Cache {
                record: Vec::new(),
                cursor: 0,
                offset: 0,
                retained: 0,
                complete: false,
                saturated: false,
            }));
        }
    }

    pub fn viable(&self) -> bool {
        self.join.viable()
    }

    pub fn frame(&self) -> usize {
        self.join.frame()
    }

    pub fn step(&mut self, index: &Index) -> Poll<Option<Vec<Slot>>> {
        if matches!(self.mode, Mode::Dormant) {
            self.mode = Mode::Visited;
        }
        let Mode::Recording(cache) = &mut self.mode else {
            return self.join.step(index);
        };
        let result = cache.step(&mut self.join, index);
        if cache.saturated {
            self.mode = Mode::Streaming;
        }
        result
    }

    pub fn evict(&mut self) {
        self.join.evict();
    }

    pub fn retained(&self) -> usize {
        self.join.retained()
            + match &self.mode {
                Mode::Recording(cache) => cache.retained + 1,
                _ => 0,
            }
    }
}

impl Cache {
    fn waiting(&self) -> usize {
        match self.record.get(self.cursor) {
            Some(Step::Waiting(count)) => count - self.offset,
            _ => 0,
        }
    }

    fn skip(&mut self, maximum: usize) -> usize {
        let available = self.waiting();
        let count = available.min(maximum);
        if count == 0 {
            return 0;
        }
        if count == available {
            self.cursor += 1;
            self.offset = 0;
        } else {
            self.offset += count;
        }
        count
    }

    fn step(&mut self, join: &mut Join, index: &Index) -> Poll<Option<Vec<Slot>>> {
        if let Some(step) = self.record.get(self.cursor) {
            match step {
                Step::Waiting(count) => {
                    self.offset += 1;
                    if self.offset == *count {
                        self.cursor += 1;
                        self.offset = 0;
                    }
                    return Poll::Pending;
                }
                Step::Binding(selection) => {
                    self.cursor += 1;
                    let selection = selection
                        .iter()
                        .map(|selection| Slot {
                            world: index.world(selection.site),
                            position: selection.position,
                            token: selection.token.clone(),
                        })
                        .collect();
                    return Poll::Ready(Some(selection));
                }
            }
        }
        if self.complete {
            return Poll::Ready(None);
        }
        let result = join.step(index);
        match &result {
            Poll::Ready(None) => self.complete = true,
            Poll::Pending => {
                if let Some(Step::Waiting(count)) = self.record.last_mut() {
                    *count += 1;
                } else if self.reserve(1) {
                    self.record.push(Step::Waiting(1));
                }
            }
            Poll::Ready(Some(selection)) => {
                let size = 1 + selection
                    .iter()
                    .map(|slot| slot.token.len() + 1)
                    .sum::<usize>();
                if self.reserve(size) {
                    let selection = selection
                        .iter()
                        .map(|slot| Selection {
                            site: index.site(slot.world),
                            position: slot.position,
                            token: slot.token.clone(),
                        })
                        .collect();
                    self.record.push(Step::Binding(selection));
                }
            }
        }
        self.cursor = self.record.len();
        result
    }

    fn reserve(&mut self, size: usize) -> bool {
        if self.retained + size > 4096 {
            self.saturated = true;
            self.record = Vec::new();
            self.retained = 0;
            return false;
        }
        self.retained += size;
        true
    }
}

#[cfg(test)]
#[path = "test/replay.rs"]
mod test;
