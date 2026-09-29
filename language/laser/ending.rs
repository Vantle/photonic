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

// Whether a run from the first of count configurations can go on forever, which a search that
// follows edges depth first finds when it meets a configuration it is still inside.
pub fn cyclic<Next: Iterator<Item = usize>>(
    count: usize,
    outgoing: impl Fn(usize) -> Next,
) -> bool {
    if count == 0 {
        return false;
    }
    let mut color = vec![0u8; count];
    let mut stack = vec![(0, outgoing(0))];
    color[0] = 1;
    while let Some((node, next)) = stack.last_mut() {
        let Some(target) = next.next() else {
            color[*node] = 2;
            stack.pop();
            continue;
        };
        match color[target] {
            0 => {
                color[target] = 1;
                stack.push((target, outgoing(target)));
            }
            1 => return true,
            _ => {}
        }
    }
    false
}
