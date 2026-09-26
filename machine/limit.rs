use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Overflow {
    Event,
    Round,
    Size,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limit {
    pub configuration: usize,
    pub coherence: usize,
    pub occurrence: usize,
    pub round: usize,
    pub terminal: usize,
    pub event: usize,
    pub individualization: usize,
    pub deadline: Option<Instant>,
}

impl Default for Limit {
    fn default() -> Self {
        Self {
            configuration: 4_096,
            coherence: 16,
            occurrence: 64,
            round: 256,
            terminal: 4,
            event: 4_096,
            individualization: 4_096,
            deadline: None,
        }
    }
}

impl Limit {
    pub fn expired(&self) -> bool {
        self.deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
    }
}
