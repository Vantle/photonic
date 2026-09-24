use network::input::Sample;
use random::Generator;
use std::collections::VecDeque;
use std::sync::Arc;

pub struct Replay {
    buffer: VecDeque<Arc<Sample>>,
    capacity: usize,
    total: u64,
}

impl Replay {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            total: 0,
        }
    }

    pub fn push(&mut self, sample: Sample) {
        if self.buffer.len() == self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(Arc::new(sample));
        self.total += 1;
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn total(&self) -> u64 {
        self.total
    }

    pub fn draw(&self, count: usize, generator: &mut Generator) -> Vec<Arc<Sample>> {
        if self.buffer.is_empty() {
            return Vec::new();
        }
        (0..count)
            .map(|_| self.buffer[generator.below(self.buffer.len())].clone())
            .collect()
    }
}
