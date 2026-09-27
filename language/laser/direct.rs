use super::{Laser, Link, map};
use crate::executor::Executor;
use hashing::Builder;
use std::collections::{HashMap, HashSet};

// An event is direct when a match found at its own source derives it, through an empty walk or a
// closed one. Empty walks are known when an event fires. A closed walk never leaves its strongly
// connected component, so the traces of cyclic components form a graph, searched from the matches
// of each configuration that still has events not known to be direct.
fn component(successor: &[Vec<usize>]) -> Vec<usize> {
    let count = successor.len();
    let mut index = vec![usize::MAX; count];
    let mut low = vec![0; count];
    let mut member = vec![false; count];
    let mut label = vec![usize::MAX; count];
    let mut stack = Vec::new();
    let mut counter = 0;
    let mut next = 0;
    for root in 0..count {
        if index[root] != usize::MAX {
            continue;
        }
        let mut work = vec![(root, 0)];
        while let Some((node, position)) = work.pop() {
            if position == 0 {
                index[node] = counter;
                low[node] = counter;
                counter += 1;
                stack.push(node);
                member[node] = true;
            }
            if let Some(&target) = successor[node].get(position) {
                work.push((node, position + 1));
                if index[target] == usize::MAX {
                    work.push((target, 0));
                } else if member[target] {
                    low[node] = low[node].min(index[target]);
                }
                continue;
            }
            if low[node] == index[node] {
                while let Some(top) = stack.pop() {
                    member[top] = false;
                    label[top] = next;
                    if top == node {
                        break;
                    }
                }
                next += 1;
            }
            if let Some(&(parent, _)) = work.last() {
                low[parent] = low[parent].min(low[node]);
            }
        }
    }
    label
}

fn condense(edge: &[Vec<usize>], label: &[usize]) -> Vec<Vec<usize>> {
    let count = label.iter().max().map_or(0, |&value| value + 1);
    let mut successor = vec![Vec::new(); count];
    for (node, list) in edge.iter().enumerate() {
        for &next in list {
            if label[node] != label[next] {
                successor[label[node]].push(label[next]);
            }
        }
    }
    for list in &mut successor {
        list.sort_unstable();
        list.dedup();
    }
    successor
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
    edge: Vec<Vec<usize>>,
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
    let mut edge = vec![Vec::new(); node.len()];
    while !frontier.is_empty() {
        let expanded = map(executor, frontier, |id| {
            let (state, position) = node[id];
            let list = laser.incoming[state]
                .iter()
                .filter_map(|&event| {
                    let value = &laser.event[event];
                    let source = value.source;
                    if label[source] != label[state] {
                        return None;
                    }
                    let found = laser.crossed[event][position]?;
                    Some(offset[source] + found.get() as usize - 1)
                })
                .collect::<Vec<_>>();
            (id, list)
        });
        frontier = Vec::new();
        for (id, list) in expanded {
            for &next in &list {
                if !std::mem::replace(&mut reached[next], true) {
                    frontier.push(next);
                }
            }
            edge[id] = list;
        }
    }
    let event = map(executor, (0..node.len()).collect(), |id| {
        let (state, position) = node[id];
        if !reached[id] || !open[state] {
            return None;
        }
        let event = match laser.link[state][position] {
            Link::Absent => None,
            Link::Event(event) => Some(event),
            Link::Unresolved => laser.find(state, &laser.trace[state][position]),
        };
        event.filter(|&event| !laser.event[event].direct)
    });
    Graph {
        offset,
        edge,
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
    let mut open = vec![false; laser.state.len()];
    for event in &laser.event {
        if !event.direct {
            open[event.source] = true;
        }
    }
    if !open.contains(&true) {
        return;
    }
    let mut successor = vec![Vec::new(); laser.state.len()];
    for event in &laser.event {
        successor[event.source].push(event.target);
    }
    let label = component(&successor);
    let cyclic = cyclic(laser, &label);
    if !cyclic
        .iter()
        .zip(&open)
        .any(|(&cyclic, &open)| cyclic && open)
    {
        return;
    }
    let graph = graph(laser, executor, &label, &cyclic, &open);
    let group = component(&graph.edge);
    let dag = condense(&graph.edge, &group);
    let origin = (0..laser.state.len())
        .filter(|&state| cyclic[state] && open[state])
        .collect::<Vec<_>>();
    let found = map(executor, origin, |state| {
        search(laser, &graph, &group, &dag, state)
    });
    for event in found.into_iter().flatten() {
        laser.event[event].direct = true;
    }
}
