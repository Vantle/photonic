use crate::setting::BLOCKED;
use crate::window::Range;
use photonic::laser::ending;
use photonic::stop::Blocked;

// What the passes of an exploration add up to: its events, the events each limit refused of the
// successors the host found, and, when cycles matter, every candidate's target, the targets of
// each marking's candidates lying from its first position to the next marking's.
#[derive(Default)]
pub struct Tally {
    pub event: u64,
    pub blocked: Blocked,
    first: Vec<u64>,
    edge: Vec<u32>,
}

impl Tally {
    // Where the targets of each marking of a pass start: its running sum within the pass, after
    // the targets of every pass before it.
    pub fn source(&mut self, start: &[u64], range: &Range) {
        let origin = self.edge.len() as u64;
        self.first.extend(
            start[range.from..range.to]
                .iter()
                .map(|&value| origin + (value - range.begin)),
        );
    }

    // The targets of a pass's candidates, in order.
    pub fn link(&mut self, target: &[u32]) {
        self.edge.extend_from_slice(target);
    }

    // Whether the edges of every marking hold a cycle the start reaches; a candidate the limits
    // refused is no edge, and a marking the budget left unexpanded has none.
    pub fn cyclic(&mut self, count: usize) -> bool {
        self.first.resize(count + 1, self.edge.len() as u64);
        ending::cyclic(count, |node| {
            self.edge[self.first[node] as usize..self.first[node + 1] as usize]
                .iter()
                .filter(|&&aim| aim != BLOCKED)
                .map(|&aim| aim as usize)
        })
    }
}
