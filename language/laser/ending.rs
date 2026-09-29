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
        let mut color = vec![0u8; self.state.len()];
        let mut stack = vec![(0, 0)];
        color[0] = 1;
        let mut endless = false;
        while let Some(&mut (node, ref mut position)) = stack.last_mut() {
            let Some(&next) = outgoing[node].get(*position) else {
                color[node] = 2;
                stack.pop();
                continue;
            };
            *position += 1;
            match color[next] {
                0 => {
                    color[next] = 1;
                    stack.push((next, 0));
                }
                1 => {
                    endless = true;
                    break;
                }
                _ => {}
            }
        }
        Ending { end, endless }
    }
}
