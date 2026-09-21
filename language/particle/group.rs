use std::sync::Arc;

pub(super) struct Group {
    pub candidate: Vec<usize>,
    pub position: Arc<Vec<usize>>,
}

impl Group {
    pub fn advance(&self, selected: &mut [usize]) -> bool {
        let width = self.position.len();
        let Some(position) = (0..width).rev().find(|&position| {
            selected[self.position[position]] < self.candidate.len() - width + position
        }) else {
            self.reset(selected);
            return false;
        };
        selected[self.position[position]] += 1;
        for index in position + 1..width {
            selected[self.position[index]] = selected[self.position[index - 1]] + 1;
        }
        true
    }

    pub fn reset(&self, selected: &mut [usize]) {
        for (index, &position) in self.position.iter().enumerate() {
            selected[position] = index;
        }
    }
}
