pub(crate) struct Graph {
    offset: Vec<usize>,
    edge: Vec<(u8, usize)>,
}

impl Graph {
    pub(crate) fn new(vertex: usize, connection: Vec<(usize, usize, u8)>) -> Self {
        let mut offset = vec![0; vertex + 1];
        for &(source, _, _) in &connection {
            offset[source + 1] += 1;
        }
        for index in 1..offset.len() {
            offset[index] += offset[index - 1];
        }
        let mut cursor = offset[..vertex].to_vec();
        let mut edge = vec![(0, 0); connection.len()];
        for (source, target, kind) in connection {
            edge[cursor[source]] = (kind, target);
            cursor[source] += 1;
        }
        Self { offset, edge }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &[(u8, usize)]> {
        self.offset
            .windows(2)
            .map(|range| &self.edge[range[0]..range[1]])
    }
}

impl std::ops::Index<usize> for Graph {
    type Output = [(u8, usize)];

    fn index(&self, index: usize) -> &Self::Output {
        &self.edge[self.offset[index]..self.offset[index + 1]]
    }
}
