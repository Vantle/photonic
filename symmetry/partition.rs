use crate::graph::{Edge, Graph, Relation};
use hashing::combine;
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub struct Partition {
    element: Vec<u32>,
    position: Vec<u32>,
    start: Vec<u32>,
    end: Vec<u32>,
    cell: usize,
}

#[derive(Clone, Copy)]
enum Direction {
    Child,
    Parent,
}

impl Direction {
    fn code(self, relation: Relation) -> u64 {
        let side = match self {
            Self::Child => 0,
            Self::Parent => 1,
        };
        side << 8 | relation as u64
    }
}

struct Refinement {
    queue: VecDeque<u32>,
    queued: Vec<bool>,
    trace: u64,
}

impl Partition {
    pub fn new(graph: &Graph) -> Self {
        let mut element = (0..graph.len() as u32).collect::<Vec<_>>();
        element.sort_by_key(|&vertex| graph.color[vertex as usize]);
        let mut partition = Self {
            element,
            position: vec![0; graph.len()],
            start: vec![0; graph.len()],
            end: vec![0; graph.len()],
            cell: 0,
        };
        let mut index = 0;
        while index < partition.element.len() {
            let color = graph.color[partition.element[index] as usize];
            let mut last = index;
            while last < partition.element.len()
                && graph.color[partition.element[last] as usize] == color
            {
                last += 1;
            }
            partition.define(index, last);
            index = last;
        }
        partition
    }

    fn define(&mut self, first: usize, last: usize) {
        for index in first..last {
            let vertex = self.element[index] as usize;
            self.position[vertex] = index as u32;
            self.start[vertex] = first as u32;
        }
        self.end[first] = last as u32;
        self.cell += 1;
    }

    pub fn element(&self) -> &[u32] {
        &self.element
    }

    pub fn position(&self, vertex: u32) -> u32 {
        self.position[vertex as usize]
    }

    pub fn cell(&self, start: u32) -> &[u32] {
        &self.element[start as usize..self.end[start as usize] as usize]
    }

    pub fn target(&self, atom: usize) -> Option<u32> {
        self.smallest(0, atom)
            .or_else(|| self.smallest(0, self.element.len()))
    }

    fn smallest(&self, first: usize, last: usize) -> Option<u32> {
        let mut best: Option<(u32, u32)> = None;
        let mut index = first;
        while index < last {
            let end = self.end[index];
            let size = end - index as u32;
            if size > 1 && best.is_none_or(|(_, least)| size < least) {
                best = Some((index as u32, size));
            }
            index = end as usize;
        }
        best.map(|(start, _)| start)
    }

    pub fn start(&self) -> Vec<u32> {
        let mut start = Vec::new();
        let mut index = 0;
        while index < self.element.len() {
            start.push(index as u32);
            index = self.end[index] as usize;
        }
        start
    }

    pub fn individualize(&mut self, vertex: u32) -> u32 {
        let first = self.start[vertex as usize];
        let last = self.end[first as usize];
        let back = last - 1;
        if back == first {
            return first;
        }
        self.swap(vertex, self.element[back as usize]);
        self.start[vertex as usize] = back;
        self.end[back as usize] = last;
        self.end[first as usize] = back;
        self.cell += 1;
        back
    }

    fn swap(&mut self, left: u32, right: u32) {
        let (from, to) = (self.position[left as usize], self.position[right as usize]);
        self.element.swap(from as usize, to as usize);
        self.position[left as usize] = to;
        self.position[right as usize] = from;
    }

    pub fn refine(&mut self, graph: &Graph, splitter: &[u32]) -> u64 {
        let mut refinement = Refinement {
            queue: VecDeque::new(),
            queued: vec![false; self.element.len()],
            trace: 0,
        };
        for &start in splitter {
            refinement.queued[start as usize] = true;
            refinement.queue.push_back(start);
        }
        let mut contribution = Vec::new();
        while let Some(start) = refinement.queue.pop_front() {
            refinement.queued[start as usize] = false;
            let member = self.cell(start).to_vec();
            for direction in [Direction::Child, Direction::Parent] {
                contribution.clear();
                for &vertex in &member {
                    let edge = match direction {
                        Direction::Child => &graph.parent[vertex as usize],
                        Direction::Parent => &graph.child[vertex as usize],
                    };
                    contribution.extend(edge.iter().copied());
                }
                contribution.sort_unstable_by_key(|edge| (edge.relation, edge.vertex));
                for group in contribution.chunk_by(|left, right| left.relation == right.relation) {
                    let tag = combine(u64::from(start), direction.code(group[0].relation));
                    self.split(group, tag, &mut refinement);
                }
            }
        }
        combine(refinement.trace, self.cell as u64)
    }

    fn split(&mut self, group: &[Edge], tag: u64, refinement: &mut Refinement) {
        let mut touched = group
            .chunk_by(|left, right| left.vertex == right.vertex)
            .map(|run| {
                let total = run.iter().map(|edge| u64::from(edge.count)).sum::<u64>();
                (self.start[run[0].vertex as usize], total, run[0].vertex)
            })
            .collect::<Vec<_>>();
        touched.sort_unstable_by_key(|&(start, total, _)| (start, total));
        for cell in touched.chunk_by(|left, right| left.0 == right.0) {
            self.divide(cell, tag, refinement);
        }
    }

    fn divide(&mut self, touched: &[(u32, u64, u32)], tag: u64, refinement: &mut Refinement) {
        let first = touched[0].0;
        let last = self.end[first as usize];
        let size = (last - first) as usize;
        if touched.len() == size && touched[0].1 == touched[touched.len() - 1].1 {
            return;
        }
        let mut back = last;
        for &(_, _, vertex) in touched {
            back -= 1;
            self.swap(vertex, self.element[back as usize]);
        }
        for (offset, &(_, _, vertex)) in touched.iter().enumerate() {
            let index = back as usize + offset;
            self.element[index] = vertex;
            self.position[vertex as usize] = index as u32;
        }
        let mut fragment = Vec::new();
        if back > first {
            fragment.push((first, back, 0));
        }
        let mut index = back;
        for run in touched.chunk_by(|left, right| left.1 == right.1) {
            fragment.push((index, index + run.len() as u32, run[0].1));
            index += run.len() as u32;
        }
        let mut trace = combine(tag, u64::from(first));
        for &(from, to, total) in &fragment {
            trace = combine(combine(trace, total), u64::from(to - from));
        }
        refinement.trace = combine(refinement.trace, trace);
        let queued = refinement.queued[first as usize];
        let largest = fragment
            .iter()
            .enumerate()
            .max_by_key(|&(position, &(from, to, _))| (to - from, std::cmp::Reverse(position)))
            .map_or(0, |(position, _)| position);
        for (position, &(from, to, _)) in fragment.iter().enumerate() {
            if from != first {
                for index in from..to {
                    self.start[self.element[index as usize] as usize] = from;
                }
                self.cell += 1;
            }
            self.end[from as usize] = to;
            if (queued || position != largest) && !refinement.queued[from as usize] {
                refinement.queued[from as usize] = true;
                refinement.queue.push_back(from);
            }
        }
    }
}
