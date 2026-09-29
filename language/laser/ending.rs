use super::Laser;
use crate::status::Status;

// A run ends at a supported configuration that no supported event leaves, and can go on forever
// when the supported configurations it passes form a cycle.
pub struct Ending {
    pub end: Vec<usize>,
    pub endless: bool,
}

impl Laser {
    pub fn ending(&self) -> Ending {
        let (state, event) = self.status();
        let mut outgoing = vec![Vec::new(); self.state.len()];
        for (index, value) in self.event.iter().enumerate() {
            if event[index] == Status::Supported {
                outgoing[value.source].push(value.target);
            }
        }
        let end = (0..self.state.len())
            .filter(|&index| state[index] == Status::Supported && outgoing[index].is_empty())
            .collect();
        let endless = cyclic(self.state.len(), |node| outgoing[node].iter().copied());
        Ending { end, endless }
    }
}

// Whether a run from the first of count configurations can go on forever.
pub fn cyclic<Next: Iterator<Item = usize>>(
    count: usize,
    outgoing: impl Fn(usize) -> Next,
) -> bool {
    cycle(count, |node| outgoing(node).map(|target| ((), target))).is_some()
}

// Where a depth-first search stands with a configuration: not reached yet, inside it, or past it.
#[derive(Clone, Copy)]
enum Visit {
    Unseen,
    Open,
    Done,
}

// A run from the first of count configurations that goes on forever, which a search following
// edges depth first finds when it meets a configuration it is still inside: that configuration,
// and the edges from the first one around to it.
pub fn cycle<Edge, Next: Iterator<Item = (Edge, usize)>>(
    count: usize,
    outgoing: impl Fn(usize) -> Next,
) -> Option<(usize, Vec<Edge>)> {
    if count == 0 {
        return None;
    }
    let mut visit = vec![Visit::Unseen; count];
    let mut stack = vec![(0, outgoing(0))];
    let mut trail = Vec::new();
    visit[0] = Visit::Open;
    while let Some((node, next)) = stack.last_mut() {
        let Some((edge, target)) = next.next() else {
            visit[*node] = Visit::Done;
            stack.pop();
            trail.pop();
            continue;
        };
        match visit[target] {
            Visit::Unseen => {
                visit[target] = Visit::Open;
                trail.push(edge);
                stack.push((target, outgoing(target)));
            }
            Visit::Open => {
                trail.push(edge);
                return Some((target, trail));
            }
            Visit::Done => {}
        }
    }
    None
}
