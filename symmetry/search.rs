use crate::component::Decomposition;
use crate::graph::Graph;
use crate::partition::Partition;
use code::forest::Forest;
use hashing::combine;
use std::cmp::Ordering;

const LEAF: u64 = 0x2545_f491_4f6c_dd1d;
// The search recurses once for each node that leaves children to explore, and WebAssembly's 1 MiB
// stack overflowed near 3,500 levels, so deeper searches stop with Exhausted::Depth instead.
const DEPTH: usize = 1024;

pub(crate) struct Labeling {
    pub element: Vec<u32>,
    pub certificate: Vec<u64>,
    pub generator: Vec<Vec<(u32, u32)>>,
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
    decomposition: &'graph Decomposition,
    path: Vec<u32>,
    trace: Vec<u64>,
    point: Vec<Vec<u32>>,
    first: Option<Leaf>,
    best: Option<Leaf>,
    revision: usize,
    generator: Vec<Vec<u32>>,
    orbit: Vec<u64>,
    node: usize,
    depth: usize,
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
    fn orbit(&self, node: &Node<'_>) -> Vec<usize> {
        let mut forest = Forest::new(node.cell.len());
        let local = |vertex: u32| (node.partition.position(vertex) - node.target) as usize;
        for generator in &self.generator {
            if self
                .path
                .iter()
                .any(|&vertex| generator[vertex as usize] != vertex)
            {
                continue;
            }
            for &vertex in node.cell {
                forest.join(local(vertex), local(generator[vertex as usize]));
            }
        }
        self.decomposition
            .join(&self.point, node.cell, local, &mut forest);
        node.cell
            .iter()
            .map(|&vertex| forest.root(local(vertex)))
            .collect()
    }

    fn standing(&self, inherited: Standing) -> Standing {
        let level = self.trace.len() - 1;
        let Some(first) = &self.first else {
            return inherited;
        };
        let best = self.best.as_ref().unwrap_or(first);
        Standing {
            first: inherited.first && first.trace.get(level) == Some(&self.trace[level]),
            best: inherited.best.then_with(|| {
                best.trace
                    .get(level)
                    .map_or(Ordering::Greater, |known| self.trace[level].cmp(known))
            }),
        }
    }

    fn enter(&mut self, vertex: u32, trace: u64) {
        self.path.push(vertex);
        self.trace.push(trace);
        self.decomposition.enter(&mut self.point, vertex);
    }

    fn leave(&mut self, level: usize) {
        while self.path.len() > level {
            let vertex = self.path.pop().expect("the path is longer than the level");
            self.decomposition.leave(&mut self.point, vertex);
        }
        self.trace.truncate(level + 1);
    }

    fn onward(&self) -> bool {
        let level = self.path.len();
        self.first
            .as_ref()
            .is_none_or(|first| first.path.len() > level && first.path[..level] == self.path[..])
    }

    fn explore(
        &mut self,
        partition: Partition,
        inherited: Standing,
    ) -> Result<Option<usize>, Exhausted> {
        if self.depth == DEPTH {
            return Err(Exhausted::Depth(DEPTH + 1));
        }
        let level = self.path.len();
        self.depth += 1;
        let jump = self.descend(partition, inherited);
        self.depth -= 1;
        self.leave(level);
        Ok(jump?.filter(|&jump| jump < level))
    }

    // A node whose children all lie in one orbit needs only its first child, so it becomes that
    // child in place, and a long run of such nodes costs neither stack nor copies of the partition.
    fn descend(
        &mut self,
        mut partition: Partition,
        mut inherited: Standing,
    ) -> Result<Option<usize>, Exhausted> {
        loop {
            self.node += 1;
            if self.node > self.budget {
                return Err(Exhausted::Node(self.node));
            }
            let target = partition.target(self.graph.atom);
            if target.is_none() {
                let last = self.trace.len() - 1;
                self.trace[last] = combine(self.trace[last], LEAF);
            }
            let standing = self.standing(inherited);
            if !standing.first && standing.best == Ordering::Greater {
                return Ok(None);
            }
            let Some(target) = target else {
                return Ok(self.leaf(&partition, standing));
            };
            let cell = partition.cell(target).to_vec();
            let node = Node {
                partition: &partition,
                target,
                cell: &cell,
            };
            if !self.decomposition.whole(&self.point, &cell) {
                let orbit = self.orbit(&node);
                if orbit.iter().any(|&root| root != orbit[0]) {
                    return self.branch(&node, standing);
                }
            }
            if self.onward() {
                self.orbit.push(cell.len() as u64);
            }
            let start = partition.individualize(cell[0]);
            let hash = partition.refine(self.graph, &[start]);
            self.enter(
                cell[0],
                combine(combine(u64::from(target), cell.len() as u64), hash),
            );
            inherited = standing;
        }
    }

    fn branch(
        &mut self,
        node: &Node<'_>,
        mut standing: Standing,
    ) -> Result<Option<usize>, Exhausted> {
        let level = self.path.len();
        let mut explored: Vec<usize> = Vec::new();
        let mut orbit = Vec::new();
        let mut known = usize::MAX;
        for (index, &vertex) in node.cell.iter().enumerate() {
            if !explored.is_empty() {
                if known != self.generator.len() {
                    orbit = self.orbit(node);
                    known = self.generator.len();
                }
                if explored.iter().any(|&other| orbit[other] == orbit[index]) {
                    continue;
                }
            }
            let mut child = node.partition.clone();
            let start = child.individualize(vertex);
            let hash = child.refine(self.graph, &[start]);
            self.enter(
                vertex,
                combine(
                    combine(u64::from(node.target), node.cell.len() as u64),
                    hash,
                ),
            );
            let revision = self.revision;
            let jump = self.explore(child, standing);
            self.leave(level);
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
            .filter(|first| first.path.len() > level && first.path[..level] == self.path[..])
            .map(|first| first.path[level]);
        if let Some(first) = first {
            let orbit = self.orbit(node);
            let position = node
                .cell
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

    fn leaf(&mut self, partition: &Partition, standing: Standing) -> Option<usize> {
        let certificate = certificate(self.graph, partition);
        let Some(first) = &self.first else {
            self.first = Some(Leaf {
                certificate,
                element: partition.element().to_vec(),
                path: self.path.clone(),
                trace: self.trace.clone(),
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
            let common = self
                .path
                .iter()
                .zip(&known.path)
                .take_while(|(left, right)| left == right)
                .count();
            let consistent = self
                .path
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
                path: self.path.clone(),
                trace: self.trace.clone(),
            });
            self.revision += 1;
        }
        None
    }
}

fn start(graph: &Graph) -> (Partition, u64) {
    let mut partition = Partition::new(graph);
    let start = partition.start();
    let hash = partition.refine(graph, &start);
    (partition, hash)
}

fn run(
    graph: &Graph,
    partition: &Partition,
    hash: u64,
    decomposition: &Decomposition,
    budget: usize,
) -> Result<Labeling, Exhausted> {
    let mut search = Search {
        graph,
        decomposition,
        path: Vec::new(),
        trace: vec![hash],
        point: decomposition.point(),
        first: None,
        best: None,
        revision: 0,
        generator: Vec::new(),
        orbit: Vec::new(),
        node: 0,
        depth: 0,
        budget,
    };
    search.explore(
        partition.clone(),
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
        generator: search
            .generator
            .iter()
            .map(|map| {
                map.iter()
                    .enumerate()
                    .filter(|&(from, &to)| from as u32 != to)
                    .map(|(from, &to)| (from as u32, to))
                    .collect()
            })
            .collect(),
        orbit: search.orbit,
        node: search.node,
    })
}

pub(crate) fn plain(graph: &Graph, budget: usize) -> Result<Labeling, Exhausted> {
    let (partition, hash) = start(graph);
    run(graph, &partition, hash, &Decomposition::default(), budget)
}

// Pruning with the components' automorphisms leaves the search's result as it was, the first leaf
// in search order with the least trace and certificate, and only visits fewer nodes to find it;
// should it still run out of budget, the plain search runs, so no graph the plain search can label
// goes unlabeled.
pub(crate) fn search(graph: &Graph, budget: usize) -> Result<Labeling, Exhausted> {
    let (partition, hash) = start(graph);
    let decomposition = Decomposition::new(graph, &partition, budget);
    let labeling = if decomposition.is_empty() {
        run(graph, &partition, hash, &decomposition, budget)?
    } else {
        match run(graph, &partition, hash, &decomposition, budget) {
            Ok(labeling) => Labeling {
                generator: labeling
                    .generator
                    .into_iter()
                    .chain(decomposition.generator())
                    .collect(),
                ..labeling
            },
            Err(_) => run(graph, &partition, hash, &Decomposition::default(), budget)?,
        }
    };
    Ok(Labeling {
        node: labeling.node + decomposition.node,
        ..labeling
    })
}
