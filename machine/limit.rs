#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Overflow {
    Event,
    Round,
    Size,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limit {
    pub state: usize,
    pub coherence: usize,
    pub cell: usize,
    pub round: usize,
    pub terminal: usize,
    pub event: usize,
    pub individualization: usize,
}

impl Default for Limit {
    fn default() -> Self {
        Self {
            state: 4_096,
            coherence: 16,
            cell: 64,
            round: 256,
            terminal: 4,
            event: 4_096,
            individualization: 4_096,
        }
    }
}
