use crate::setting::BLOCKED;
use photonic::laser::ending;

// What the passes of an exploration add up to: its events and, when cycles matter, every
// candidate's target, the targets of each marking's candidates lying from its first position to
// the next marking's.
#[derive(Default)]
pub struct Tally {
    pub event: u64,
    pub first: Vec<u64>,
    pub edge: Vec<u32>,
}

impl Tally {
    // Whether the edges of every marking hold a cycle the start reaches; a candidate the limits
    // refused is no edge.
    pub fn cyclic(&mut self) -> bool {
        self.first.push(self.edge.len() as u64);
        ending::cyclic(self.first.len() - 1, |node| {
            self.edge[self.first[node] as usize..self.first[node + 1] as usize]
                .iter()
                .filter(|&&aim| aim != BLOCKED)
                .map(|&aim| aim as usize)
        })
    }
}
