#[derive(Default)]
pub(crate) struct Index {
    value: Vec<Option<usize>>,
    tree: Vec<usize>,
    length: usize,
}

impl Index {
    pub fn insert(&mut self, value: usize) -> usize {
        let position = self.value.len();
        if self.tree.is_empty() {
            self.value.push(Some(value));
            self.length += 1;
            if self.value.len() > 32 {
                self.rebuild();
            }
            return position;
        }
        let end = position + 1;
        let start = end & (end - 1);
        let count = self.rank(position) - self.rank(start) + 1;
        self.value.push(Some(value));
        self.tree.push(count);
        self.length += 1;
        position
    }

    pub fn remove(&mut self, position: usize) {
        assert!(self.value[position].take().is_some());
        self.length -= 1;
        let mut cursor = position + 1;
        while cursor <= self.tree.len() {
            self.tree[cursor - 1] -= 1;
            cursor += cursor.isolate_lowest_one();
        }
    }

    pub fn rank(&self, position: usize) -> usize {
        if self.tree.is_empty() {
            if self.value.len() == self.length {
                return position;
            }
            return self.value[..position].iter().flatten().count();
        }
        let mut cursor = position;
        let mut rank = 0;
        while cursor != 0 {
            rank += self.tree[cursor - 1];
            cursor &= cursor - 1;
        }
        rank
    }

    pub fn select(&self, rank: usize) -> usize {
        assert!(rank < self.length);
        if self.tree.is_empty() {
            if self.value.len() == self.length {
                return self.value[rank].unwrap();
            }
            return *self.value.iter().flatten().nth(rank).unwrap();
        }
        let mut remaining = rank;
        let mut position = 0;
        let mut stride = self.tree.len().next_power_of_two();
        while stride != 0 {
            let next = position + stride;
            if next <= self.tree.len() && self.tree[next - 1] <= remaining {
                remaining -= self.tree[next - 1];
                position = next;
            }
            stride >>= 1;
        }
        self.value[position].unwrap()
    }

    pub fn compact(&mut self, position: &mut [usize]) {
        if !self.tree.is_empty() && self.length > 16 && self.value.len() <= self.length * 2 + 64 {
            return;
        }
        self.value.retain(Option::is_some);
        self.tree.clear();
        for (index, &value) in self.value.iter().enumerate() {
            position[value.unwrap()] = index;
        }
        if self.length > 32 {
            self.rebuild();
        }
    }

    fn rebuild(&mut self) {
        self.tree = self
            .value
            .iter()
            .map(|value| usize::from(value.is_some()))
            .collect();
        for index in 1..=self.tree.len() {
            let parent = index + index.isolate_lowest_one();
            if parent <= self.tree.len() {
                self.tree[parent - 1] += self.tree[index - 1];
            }
        }
    }

    pub fn retained(&self) -> usize {
        self.value.len() + self.tree.len()
    }
}
