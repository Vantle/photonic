use crate::graph::Graph;
use crate::partition::Partition;
use code::forest::Forest;
use code::hashing::combine;
use std::cmp::Ordering;

const LEAF: u64 = 0x2545_f491_4f6c_dd1d;
// The search recurses once per individualized cell, and WebAssembly's 1 MiB stack overflowed near
// 3,500 levels, so deeper searches stop with Exhausted::Depth instead.
const DEPTH: usize = 1024;

pub(crate) struct Labeling {
    pub element: Vec<u32>,
    pub certificate: Vec<u64>,
    pub generator: Vec<Vec<u32>>,
    pub orbit: Vec<u64>,
    pub node: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Exhausted {
    Node(usize),
    Depth(usize),
}

struct Leaf {
    certificate: Vec<u64>,
    element: Vec<u32>,
    path: Vec<u32>,
    trace: Vec<u64>,
}

#[derive(Clone, Copy)]
struct Standing {
    first: bool,
    best: Ordering,
}

struct Search<'graph> {
    graph: &'graph Graph,
    first: Option<Leaf>,
    best: Option<Leaf>,
    revision: usize,
    generator: Vec<Vec<u32>>,
    orbit: Vec<u64>,
    node: usize,
    budget: usize,
}

struct Node<'node> {
    partition: &'node Partition,
    target: u32,
    cell: &'node [u32],
}

fn certificate(graph: &Graph, partition: &Partition) -> Vec<u64> {
    let mut word = Vec::with_capacity(graph.len() * 4);
    let mut edge = Vec::new();
    for &vertex in partition.element() {
        word.extend(graph.color[vertex as usize].code());
        edge.clear();
        edge.extend(graph.child[vertex as usize].iter().map(|entry| {
            (
                partition.position(entry.vertex),
                entry.relation as u64,
                entry.count,
            )
        }));
        edge.sort_unstable();
        word.push(edge.len() as u64);
        for &(position, relation, count) in &edge {
            word.extend([u64::from(position), relation << 32 | u64::from(count)]);
        }
    }
    word
}

impl Search<'_> {
    fn orbit(&self, path: &[u32], node: &Node<'_>) -> Vec<usize> {
        let mut forest = Forest::new(node.cell.len());
        let local = |vertex: u32| (node.partition.position(vertex) - node.target) as usize;
        for generator in &self.generator {
            if path
                .iter()
                .any(|&vertex| generator[vertex as usize] != vertex)
            {
                continue;
            }
            for &vertex in node.cell {
                forest.join(local(vertex), local(generator[vertex as usize]));
            }
        }
        node.cell
            .iter()
            .map(|&vertex| forest.root(local(vertex)))
            .collect()
    }

    fn standing(&self, trace: &[u64], inherited: Standing) -> Standing {
        let level = trace.len() - 1;
        let Some(first) = &self.first else {
            return inherited;
        };
        let best = self.best.as_ref().unwrap_or(first);
        Standing {
            first: inherited.first && first.trace.get(level) == Some(&trace[level]),
            best: inherited.best.then_with(|| {
                best.trace
                    .get(level)
                    .map_or(Ordering::Greater, |known| trace[level].cmp(known))
            }),
        }
    }

    fn explore(
        &mut self,
        partition: Partition,
        path: &mut Vec<u32>,
        trace: &mut Vec<u64>,
        inherited: Standing,
    ) -> Result<Option<usize>, Exhausted> {
        self.node += 1;
        if self.node > self.budget {
            return Err(Exhausted::Node(self.node));
        }
        if path.len() > DEPTH {
            return Err(Exhausted::Depth(path.len()));
        }
        let target = partition.target(self.graph.atom);
        if target.is_none() {
            let last = trace.len() - 1;
            trace[last] = combine(trace[last], LEAF);
        }
        let mut standing = self.standing(trace, inherited);
        if !standing.first && standing.best == Ordering::Greater {
            return Ok(None);
        }
        let Some(target) = target else {
            return Ok(self.leaf(&partition, path, trace, standing));
        };
        let level = path.len();
        let cell = partition.cell(target).to_vec();
        let node = Node {
            partition: &partition,
            target,
            cell: &cell,
        };
        let mut explored: Vec<usize> = Vec::new();
        let mut orbit = Vec::new();
        let mut known = usize::MAX;
        for (index, &vertex) in cell.iter().enumerate() {
            if !explored.is_empty() {
                if known != self.generator.len() {
                    orbit = self.orbit(path, &node);
                    known = self.generator.len();
                }
                if explored.iter().any(|&other| orbit[other] == orbit[index]) {
                    continue;
                }
            }
            let mut child = partition.clone();
            let start = child.individualize(vertex);
            let hash = child.refine(self.graph, &[start]);
            path.push(vertex);
            trace.push(combine(combine(u64::from(target), cell.len() as u64), hash));
            let revision = self.revision;
            let jump = self.explore(child, path, trace, standing);
            trace.pop();
            path.pop();
            if self.revision != revision {
                standing.best = Ordering::Equal;
            }
            explored.push(index);
            if let Some(jump) = jump?
                && jump < level
            {
                return Ok(Some(jump));
            }
        }
        let first = self
            .first
            .as_ref()
            .filter(|first| first.path.len() > level && first.path[..level] == path[..])
            .map(|first| first.path[level]);
        if let Some(first) = first {
            let orbit = self.orbit(path, &node);
            let position = cell
                .iter()
                .position(|&vertex| vertex == first)
                .expect("the first path continues through its target cell");
            self.orbit.push(
                orbit
                    .iter()
                    .filter(|&&root| root == orbit[position])
                    .count() as u64,
            );
        }
        Ok(None)
    }

    fn leaf(
        &mut self,
        partition: &Partition,
        path: &[u32],
        trace: &[u64],
        standing: Standing,
    ) -> Option<usize> {
        let certificate = certificate(self.graph, partition);
        let Some(first) = &self.first else {
            self.first = Some(Leaf {
                certificate,
                element: partition.element().to_vec(),
                path: path.to_vec(),
                trace: trace.to_vec(),
            });
            return None;
        };
        let best = self.best.as_ref().unwrap_or(first);
        for (equal, known) in [(standing.first, first), (standing.best.is_eq(), best)] {
            if !equal || certificate != known.certificate {
                continue;
            }
            let mut map = vec![0; self.graph.len()];
            for (&from, &to) in partition.element().iter().zip(&known.element) {
                map[from as usize] = to;
            }
            let common = path
                .iter()
                .zip(&known.path)
                .take_while(|(left, right)| left == right)
                .count();
            let consistent = path
                .iter()
                .zip(&known.path)
                .take(common + 1)
                .all(|(&from, &to)| map[from as usize] == to);
            self.generator.push(map);
            return consistent.then_some(common);
        }
        if standing.best == Ordering::Less
            || (standing.best.is_eq() && certificate < best.certificate)
        {
            self.best = Some(Leaf {
                certificate,
                element: partition.element().to_vec(),
                path: path.to_vec(),
                trace: trace.to_vec(),
            });
            self.revision += 1;
        }
        None
    }
}

pub(crate) fn search(graph: &Graph, budget: usize) -> Result<Labeling, Exhausted> {
    let mut partition = Partition::new(graph);
    let start = partition.start();
    let hash = partition.refine(graph, &start);
    let mut search = Search {
        graph,
        first: None,
        best: None,
        revision: 0,
        generator: Vec::new(),
        orbit: Vec::new(),
        node: 0,
        budget,
    };
    search.explore(
        partition,
        &mut Vec::new(),
        &mut vec![hash],
        Standing {
            first: true,
            best: Ordering::Equal,
        },
    )?;
    let first = search.first.take().expect("every search reaches a leaf");
    let best = search.best.take().unwrap_or(first);
    Ok(Labeling {
        element: best.element,
        certificate: best.certificate,
        generator: search.generator,
        orbit: search.orbit,
        node: search.node,
    })
}
