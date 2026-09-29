// What the passes of an exploration add up to: its events, whether a limit refused anything, and,
// when cycles matter, every marking's edges, the targets of each marking's edges lying from its
// first position to the next marking's.
#[derive(Default)]
pub struct Tally {
    pub event: u64,
    pub refused: bool,
    pub first: Vec<u64>,
    pub edge: Vec<u32>,
}

impl Tally {
    // Ends the edges of every marking before this one.
    pub fn reach(&mut self, marking: usize) {
        while self.first.len() <= marking {
            self.first.push(self.edge.len() as u64);
        }
    }

    // Whether the edges of the first count markings hold a cycle the start reaches, found by a
    // search that follows edges depth first and meets a marking it is still inside.
    pub fn cyclic(&mut self, count: usize) -> bool {
        self.reach(count);
        if count == 0 {
            return false;
        }
        let mut color = vec![0u8; count];
        let mut stack = vec![(0usize, self.first[0])];
        color[0] = 1;
        while let Some(&mut (node, ref mut position)) = stack.last_mut() {
            if *position == self.first[node + 1] {
                color[node] = 2;
                stack.pop();
                continue;
            }
            let next = self.edge[*position as usize] as usize;
            *position += 1;
            match color[next] {
                0 => {
                    color[next] = 1;
                    stack.push((next, self.first[next]));
                }
                1 => return true,
                _ => {}
            }
        }
        false
    }
}
