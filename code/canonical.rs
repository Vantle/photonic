use crate::hashing::{combine, mix, value};
use std::hash::Hash;

const INDIVIDUAL: u64 = 0x5bd1_e995_7f4a_7c15;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Key<Element> {
    shape: Vec<u32>,
    element: Vec<Element>,
}

pub fn key<Element: Clone + Hash + Ord>(
    coherence: &[Vec<(u32, Element)>],
    budget: usize,
) -> Key<Element> {
    let membership = membership(coherence);
    if membership.is_empty() {
        return plain(coherence);
    }
    let graph = Graph::new(coherence, &membership);
    let initial = graph.initial();
    let mut search = Search {
        graph: &graph,
        remaining: budget,
        first: None,
        best: None,
        symmetry: Vec::new(),
    };
    search.explore(initial.clone(), &mut Vec::new());
    search.best.map_or_else(
        || graph.encode(&graph.order(&graph.refine(initial))),
        |leaf| leaf.key,
    )
}

pub fn membership<Element>(coherence: &[Vec<(u32, Element)>]) -> Vec<(u32, Vec<usize>)> {
    let mut pair = coherence
        .iter()
        .enumerate()
        .flat_map(|(index, entry)| entry.iter().map(move |(id, _)| (*id, index)))
        .collect::<Vec<_>>();
    pair.sort_unstable();
    pair.chunk_by(|left, right| left.0 == right.0)
        .filter(|chunk| chunk.len() > 1)
        .map(|chunk| (chunk[0].0, chunk.iter().map(|&(_, index)| index).collect()))
        .collect()
}

fn plain<Element: Clone + Ord>(coherence: &[Vec<(u32, Element)>]) -> Key<Element> {
    let mut sorted = coherence
        .iter()
        .map(|entry| {
            let mut element = entry
                .iter()
                .map(|(_, element)| element.clone())
                .collect::<Vec<_>>();
            element.sort_unstable();
            element
        })
        .collect::<Vec<_>>();
    sorted.sort_unstable();
    let mut shape = vec![sorted.len() as u32, 0];
    shape.extend(sorted.iter().map(|element| element.len() as u32));
    Key {
        shape,
        element: sorted.into_iter().flatten().collect(),
    }
}

struct Node<Element> {
    private: Vec<Element>,
    link: Vec<usize>,
}

struct Link<Element> {
    element: Element,
    member: Vec<usize>,
}

struct Graph<Element> {
    node: Vec<Node<Element>>,
    link: Vec<Link<Element>>,
}

#[derive(Clone)]
struct Coloring {
    node: Vec<u64>,
    link: Vec<u64>,
}

#[derive(Clone)]
struct Leaf<Element> {
    key: Key<Element>,
    order: Vec<usize>,
    path: Vec<usize>,
}

struct Search<'graph, Element> {
    graph: &'graph Graph<Element>,
    remaining: usize,
    first: Option<Leaf<Element>>,
    best: Option<Leaf<Element>>,
    symmetry: Vec<Vec<usize>>,
}

fn distinct(color: &[u64], buffer: &mut Vec<u64>) -> usize {
    buffer.clear();
    buffer.extend_from_slice(color);
    buffer.sort_unstable();
    buffer.dedup();
    buffer.len()
}

impl Coloring {
    fn classes(&self, buffer: &mut Vec<u64>) -> usize {
        distinct(&self.node, buffer) + distinct(&self.link, buffer)
    }
}

fn fold(seed: u64, color: impl Iterator<Item = u64>) -> u64 {
    combine(
        seed,
        color.fold(0u64, |total, value| total.wrapping_add(mix(value))),
    )
}

impl<Element: Clone + Hash + Ord> Graph<Element> {
    fn new(coherence: &[Vec<(u32, Element)>], membership: &[(u32, Vec<usize>)]) -> Self {
        let shared = |id: u32| membership.binary_search_by_key(&id, |entry| entry.0).ok();
        let node = coherence
            .iter()
            .map(|entry| {
                let mut private = entry
                    .iter()
                    .filter(|(id, _)| shared(*id).is_none())
                    .map(|(_, element)| element.clone())
                    .collect::<Vec<_>>();
                private.sort_unstable();
                Node {
                    private,
                    link: entry.iter().filter_map(|(id, _)| shared(*id)).collect(),
                }
            })
            .collect();
        let link = membership
            .iter()
            .map(|(id, member)| Link {
                element: coherence[member[0]]
                    .iter()
                    .find(|(entry, _)| entry == id)
                    .map(|(_, element)| element.clone())
                    .expect("a shared introduction appears in its first member"),
                member: member.clone(),
            })
            .collect();
        Self { node, link }
    }

    fn initial(&self) -> Coloring {
        Coloring {
            node: self
                .node
                .iter()
                .map(|node| {
                    let mut element = node
                        .link
                        .iter()
                        .map(|&link| value(&self.link[link].element))
                        .collect::<Vec<_>>();
                    element.sort_unstable();
                    value(&(&node.private, element))
                })
                .collect(),
            link: self
                .link
                .iter()
                .map(|link| value(&(&link.element, link.member.len())))
                .collect(),
        }
    }

    fn refine(&self, coloring: Coloring) -> Coloring {
        let mut buffer = Vec::with_capacity(self.node.len().max(self.link.len()));
        let mut current = coloring;
        let mut classes = current.classes(&mut buffer);
        loop {
            let node = self
                .node
                .iter()
                .enumerate()
                .map(|(index, node)| {
                    fold(
                        current.node[index],
                        node.link.iter().map(|&link| current.link[link]),
                    )
                })
                .collect::<Vec<_>>();
            let link = self
                .link
                .iter()
                .enumerate()
                .map(|(index, link)| {
                    fold(
                        current.link[index],
                        link.member.iter().map(|&member| node[member]),
                    )
                })
                .collect();
            current = Coloring { node, link };
            let refined = current.classes(&mut buffer);
            if refined == classes {
                return current;
            }
            classes = refined;
        }
    }

    fn order(&self, coloring: &Coloring) -> Vec<usize> {
        let mut order = (0..self.node.len()).collect::<Vec<_>>();
        order.sort_by_key(|&index| (coloring.node[index], index));
        order
    }

    fn encode(&self, order: &[usize]) -> Key<Element> {
        let mut position = vec![0u32; order.len()];
        for (rank, &index) in order.iter().enumerate() {
            position[index] = rank as u32;
        }
        let mut link = self
            .link
            .iter()
            .enumerate()
            .map(|(index, link)| {
                let mut member = link
                    .member
                    .iter()
                    .map(|&member| position[member])
                    .collect::<Vec<_>>();
                member.sort_unstable();
                (link.element.clone(), member, index)
            })
            .collect::<Vec<_>>();
        link.sort_unstable();
        let mut rank = vec![0u32; link.len()];
        for (value, (_, _, index)) in link.iter().enumerate() {
            rank[*index] = value as u32;
        }
        let mut shape = vec![order.len() as u32, link.len() as u32];
        let mut element = link
            .iter()
            .map(|(element, _, _)| element.clone())
            .collect::<Vec<_>>();
        for &index in order {
            let node = &self.node[index];
            let mut reference = node.link.iter().map(|&link| rank[link]).collect::<Vec<_>>();
            reference.sort_unstable();
            shape.push(node.private.len() as u32);
            shape.push(reference.len() as u32);
            shape.extend(reference);
            element.extend(node.private.iter().cloned());
        }
        Key { shape, element }
    }
}

fn repeated(color: &[u64]) -> Option<u64> {
    let mut sorted = color.to_vec();
    sorted.sort_unstable();
    sorted
        .windows(2)
        .find(|pair| pair[0] == pair[1])
        .map(|pair| pair[0])
}

fn find(parent: &mut [usize], node: usize) -> usize {
    let mut root = node;
    while parent[root] != root {
        root = parent[root];
    }
    let mut current = node;
    while parent[current] != root {
        let next = parent[current];
        parent[current] = root;
        current = next;
    }
    root
}

fn automorphism(order: &[usize], target: &[usize]) -> Vec<usize> {
    let mut map = vec![0; order.len()];
    for (&from, &to) in order.iter().zip(target) {
        map[from] = to;
    }
    map
}

fn divergence(map: &[usize], path: &[usize], target: &[usize]) -> Option<usize> {
    let common = path
        .iter()
        .zip(target)
        .take_while(|(left, right)| left == right)
        .count();
    path.iter()
        .zip(target)
        .take(common + 1)
        .all(|(&from, &to)| map[from] == to)
        .then_some(common)
}

impl<Element: Clone + Hash + Ord> Search<'_, Element> {
    fn explore(&mut self, coloring: Coloring, path: &mut Vec<usize>) -> Option<usize> {
        let coloring = self.graph.refine(coloring);
        let Some(cell) = repeated(&coloring.node) else {
            return self.leaf(&coloring, path);
        };
        let level = path.len();
        let mut explored = Vec::new();
        for member in 0..self.graph.node.len() {
            if coloring.node[member] != cell {
                continue;
            }
            if self.remaining == 0 {
                return Some(0);
            }
            if self.equivalent(member, &explored, path) {
                continue;
            }
            self.remaining -= 1;
            let mut next = coloring.clone();
            next.node[member] = combine(cell, INDIVIDUAL);
            path.push(member);
            let jump = self.explore(next, path);
            path.pop();
            explored.push(member);
            if let Some(target) = jump
                && target < level
            {
                return Some(target);
            }
        }
        None
    }

    fn leaf(&mut self, coloring: &Coloring, path: &[usize]) -> Option<usize> {
        let order = self.graph.order(coloring);
        let key = self.graph.encode(&order);
        let Some(first) = &self.first else {
            let leaf = Leaf {
                key,
                order,
                path: path.to_vec(),
            };
            self.best = Some(leaf.clone());
            self.first = Some(leaf);
            return None;
        };
        for known in [Some(first), self.best.as_ref()].into_iter().flatten() {
            if key != known.key {
                continue;
            }
            let map = automorphism(&order, &known.order);
            let jump = divergence(&map, path, &known.path);
            self.symmetry.push(map);
            return jump;
        }
        if self.best.as_ref().is_some_and(|best| key < best.key) {
            self.best = Some(Leaf {
                key,
                order,
                path: path.to_vec(),
            });
        }
        None
    }

    fn equivalent(&self, member: usize, explored: &[usize], path: &[usize]) -> bool {
        if explored.is_empty() || self.symmetry.is_empty() {
            return false;
        }
        let mut parent = (0..self.graph.node.len()).collect::<Vec<_>>();
        for map in &self.symmetry {
            if path.iter().any(|&vertex| map[vertex] != vertex) {
                continue;
            }
            for (from, &to) in map.iter().enumerate() {
                let (left, right) = (find(&mut parent, from), find(&mut parent, to));
                parent[left.max(right)] = left.min(right);
            }
        }
        let root = find(&mut parent, member);
        explored
            .iter()
            .any(|&other| find(&mut parent, other) == root)
    }
}
