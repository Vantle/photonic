use std::collections::BTreeMap;

pub struct Forest {
    parent: Vec<usize>,
}

impl Forest {
    pub fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
        }
    }

    pub fn root(&mut self, index: usize) -> usize {
        let mut root = index;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut node = index;
        while node != root {
            let next = self.parent[node];
            self.parent[node] = root;
            node = next;
        }
        root
    }

    pub fn join(&mut self, left: usize, right: usize) {
        let left = self.root(left);
        let right = self.root(right);
        self.parent[left.max(right)] = left.min(right);
    }

    pub fn group(&mut self) -> Vec<Vec<usize>> {
        let mut group = BTreeMap::<usize, Vec<usize>>::new();
        for index in 0..self.parent.len() {
            let root = self.root(index);
            group.entry(root).or_default().push(index);
        }
        group.into_values().collect()
    }
}
