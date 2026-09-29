use super::{CHUNK, Laser, map};
use crate::executor::Executor;
use crate::profile;
use hashing::Builder;
use std::collections::{HashMap, HashSet};

// An event is direct when a match found at its own source derives it, through an empty walk or a
// closed one. Empty walks are known when an event fires. A closed walk never leaves its strongly
// connected component, so the traces of cyclic components form a graph, searched from the matches
// of each configuration that still has events not known to be direct. The graph has an edge for
// every crossing, so its nodes are numbered in 32 bits.
fn number(value: usize) -> u32 {
    u32::try_from(value).expect("fewer than 2^32 nodes")
}

// A graph's edges kept in one list, each node's edges lying in the span it keeps, so reading them
// follows no pointer per node; a node with no edges keeps an empty span.
struct Adjacency {
    span: Vec<(usize, usize)>,
    target: Vec<u32>,
}

impl Adjacency {
    fn edge(&self, node: usize) -> &[u32] {
        let (start, end) = self.span[node];
        &self.target[start..end]
    }
}

// The strongly connected components of a graph, by Tarjan's algorithm, as each node's component.
// Each node on the path the search follows keeps the next of its edges to read, and a node the
// search reached but gave no component yet is still on the stack.
fn component(graph: &Adjacency) -> Vec<usize> {
    const UNSEEN: u32 = u32::MAX;
    let count = graph.span.len();
    let mut index = vec![UNSEEN; count];
    let mut low = vec![0; count];
    let mut label = vec![usize::MAX; count];
    let mut stack = Vec::new();
    let mut path = Vec::<(usize, usize)>::new();
    let mut counter = 0;
    let mut next = 0;
    for root in 0..count {
        if index[root] != UNSEEN {
            continue;
        }
        index[root] = counter;
        low[root] = counter;
        counter += 1;
        stack.push(root);
        path.push((root, graph.span[root].0));
        while let Some((node, position)) = path.last_mut() {
            let node = *node;
            if *position < graph.span[node].1 {
                let target = graph.target[*position] as usize;
                *position += 1;
                if index[target] == UNSEEN {
                    index[target] = counter;
                    low[target] = counter;
                    counter += 1;
                    stack.push(target);
                    path.push((target, graph.span[target].0));
                } else if label[target] == usize::MAX {
                    low[node] = low[node].min(index[target]);
                }
                continue;
            }
            path.pop();
            if low[node] == index[node] {
                while let Some(top) = stack.pop() {
                    label[top] = next;
                    if top == node {
                        break;
                    }
                }
                next += 1;
            }
            if let Some(&(parent, _)) = path.last() {
                low[parent] = low[parent].min(low[node]);
            }
        }
    }
    label
}

fn condense(graph: &Adjacency, label: &[usize]) -> Vec<Vec<usize>> {
    let count = label.iter().max().map_or(0, |&value| value + 1);
    let mut successor = vec![Vec::new(); count];
    for (node, &value) in label.iter().enumerate() {
        for &next in graph.edge(node) {
            let next = label[next as usize];
            if value != next {
                successor[value].push(next);
            }
        }
    }
    for list in &mut successor {
        list.sort_unstable();
        list.dedup();
    }
    successor
}

// Configurations and the events between them, each configuration's edges leading to its events'
// targets.
fn successor(laser: &Laser) -> Adjacency {
    let count = laser.state.len();
    let mut first = vec![0; count + 1];
    for event in &laser.event {
        first[event.source + 1] += 1;
    }
    for node in 0..count {
        first[node + 1] += first[node];
    }
    let mut fill = first.clone();
    let mut target = vec![0; laser.event.len()];
    for event in &laser.event {
        target[fill[event.source]] = number(event.target);
        fill[event.source] += 1;
    }
    Adjacency {
        span: (0..count)
            .map(|node| (first[node], first[node + 1]))
            .collect(),
        target,
    }
}

fn cyclic(laser: &Laser, label: &[usize]) -> Vec<bool> {
    let mut size = vec![0; laser.state.len()];
    for &value in label {
        size[value] += 1;
    }
    let mut cyclic = label
        .iter()
        .map(|&value| size[value] > 1)
        .collect::<Vec<_>>();
    for event in &laser.event {
        if event.source == event.target {
            cyclic[event.target] = true;
        }
    }
    cyclic
}

struct Graph {
    offset: Vec<usize>,
    edge: Adjacency,
    event: Vec<Option<usize>>,
}

fn graph(
    laser: &Laser,
    executor: Option<&Executor>,
    label: &[usize],
    cyclic: &[bool],
    open: &[bool],
) -> Graph {
    let mut offset = vec![usize::MAX; laser.state.len()];
    let mut node = Vec::new();
    let mut frontier = Vec::new();
    for state in (0..laser.state.len()).filter(|&state| cyclic[state]) {
        offset[state] = node.len();
        if open[state] {
            frontier.extend(node.len()..node.len() + laser.origin[state]);
        }
        node.extend((0..laser.trace[state].len()).map(|position| (state, position)));
    }
    let mut reached = vec![false; node.len()];
    for &id in &frontier {
        reached[id] = true;
    }
    let mut span = vec![(0, 0); node.len()];
    let mut target = Vec::new();
    while !frontier.is_empty() {
        let chunk = frontier.chunks(CHUNK).map(<[usize]>::to_vec).collect();
        let expanded = map(executor, chunk, |chunk: Vec<usize>| {
            let mut edge = Vec::new();
            let mut length = Vec::with_capacity(chunk.len());
            for &id in &chunk {
                let before = edge.len();
                let (state, position) = node[id];
                edge.extend(laser.incoming[state].iter().filter_map(|&event| {
                    let source = laser.event[event].source;
                    if label[source] != label[state] {
                        return None;
                    }
                    let landed = laser.landing(event, position)?;
                    Some(number(offset[source] + landed))
                }));
                length.push(edge.len() - before);
            }
            (chunk, length, edge)
        });
        frontier = Vec::new();
        for (chunk, length, edge) in expanded {
            let mut start = target.len();
            for (&id, &length) in chunk.iter().zip(&length) {
                span[id] = (start, start + length);
                start += length;
            }
            for &next in &edge {
                let next = next as usize;
                if !std::mem::replace(&mut reached[next], true) {
                    frontier.push(next);
                }
            }
            target.extend(edge);
        }
    }
    let event = map(executor, (0..node.len()).collect(), |id| {
        let (state, position) = node[id];
        if !reached[id] || !open[state] {
            return None;
        }
        laser
            .linked(state, position)
            .filter(|&event| !laser.event[event].direct)
    });
    Graph {
        offset,
        edge: Adjacency { span, target },
        event,
    }
}

fn search(
    laser: &Laser,
    graph: &Graph,
    group: &[usize],
    dag: &[Vec<usize>],
    state: usize,
) -> Vec<usize> {
    let start = graph.offset[state];
    let mut wanted = HashMap::<usize, Vec<usize>, Builder>::default();
    let own = start..start + laser.trace[state].len();
    for (&event, &value) in graph.event[own.clone()].iter().zip(&group[own]) {
        if let Some(event) = event {
            wanted.entry(value).or_default().push(event);
        }
    }
    let mut found = Vec::new();
    if wanted.is_empty() {
        return found;
    }
    let mut stack = (start..start + laser.origin[state])
        .map(|id| group[id])
        .collect::<Vec<_>>();
    stack.sort_unstable();
    stack.dedup();
    let mut seen = stack.iter().copied().collect::<HashSet<usize, Builder>>();
    while let Some(value) = stack.pop() {
        if let Some(list) = wanted.remove(&value) {
            found.extend(list);
            if wanted.is_empty() {
                return found;
            }
        }
        for &next in &dag[value] {
            if seen.insert(next) {
                stack.push(next);
            }
        }
    }
    found
}

pub(super) fn mark(laser: &mut Laser, executor: Option<&Executor>) {
    let _scope = profile::Scope::new(profile::Phase::Marking);
    let mut open = vec![false; laser.state.len()];
    for event in &laser.event {
        if !event.direct {
            open[event.source] = true;
        }
    }
    if !open.contains(&true) {
        return;
    }
    let label = component(&successor(laser));
    let cyclic = cyclic(laser, &label);
    let start = (0..laser.state.len())
        .filter(|&state| cyclic[state] && open[state])
        .collect::<Vec<_>>();
    if start.is_empty() {
        return;
    }
    let graph = graph(laser, executor, &label, &cyclic, &open);
    let group = component(&graph.edge);
    let dag = condense(&graph.edge, &group);
    let found = map(executor, start, |state| {
        search(laser, &graph, &group, &dag, state)
    });
    for event in found.into_iter().flatten() {
        laser.event[event].direct = true;
    }
}
