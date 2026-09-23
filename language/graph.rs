use crate::link::Link;

pub(crate) struct Graph {
    offset: Vec<usize>,
    edge: Vec<(Link, usize)>,
}

impl Graph {
    pub(crate) fn new(
        vertex: usize,
        connection: impl Iterator<Item = (usize, usize, Link)> + Clone,
    ) -> Self {
        let mut offset = vec![0; vertex + 1];
        for (source, _, _) in connection.clone() {
            offset[source + 1] += 1;
        }
        for index in 1..offset.len() {
            offset[index] += offset[index - 1];
        }
        let mut cursor = offset[..vertex].to_vec();
        let mut edge = vec![(Link::Context, 0); offset[vertex]];
        for (source, target, kind) in connection {
            edge[cursor[source]] = (kind, target);
            cursor[source] += 1;
        }
        Self { offset, edge }
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &[(Link, usize)]> + Clone {
        self.offset
            .windows(2)
            .map(|range| &self.edge[range[0]..range[1]])
    }
}

impl std::ops::Index<usize> for Graph {
    type Output = [(Link, usize)];

    fn index(&self, index: usize) -> &Self::Output {
        &self.edge[self.offset[index]..self.offset[index + 1]]
    }
}
