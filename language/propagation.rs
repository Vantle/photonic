use crate::accumulator::Accumulator;
use crate::fingerprint::mix;
use crate::incidence::Label;
use std::hash::{Hash, Hasher};

#[derive(Default)]
struct Vertex {
    active: bool,
    edge: Vec<(u8, usize)>,
    color: [u64; 5],
    summary: [Accumulator; 4],
    dirty: [bool; 4],
}

#[derive(Default)]
pub(crate) struct Network {
    vertex: Vec<Vertex>,
    vacant: Vec<usize>,
    pending: [Vec<usize>; 4],
    summary: Accumulator,
    edge: usize,
}

fn label(value: Label) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hash);
    hash.finish()
}

impl Network {
    fn mark(&mut self, vertex: usize, phase: usize) {
        if !std::mem::replace(&mut self.vertex[vertex].dirty[phase], true) {
            self.pending[phase].push(vertex);
        }
    }

    pub fn insert(&mut self, value: Label) -> usize {
        let index = self.vacant.pop().unwrap_or_else(|| {
            self.vertex.push(Vertex::default());
            self.vertex.len() - 1
        });
        self.vertex[index] = Vertex {
            active: true,
            color: [label(value), 0, 0, 0, 0],
            ..Vertex::default()
        };
        self.summary.insert(0);
        for phase in 0..4 {
            self.mark(index, phase);
        }
        index
    }

    pub fn connect(&mut self, source: usize, target: usize, kind: u8) {
        self.vertex[source].edge.push((kind, target));
        self.vertex[target].edge.push((kind + 1, source));
        self.edge += 2;
        for phase in 0..4 {
            let forward = mix(self.vertex[target].color[phase].wrapping_add(mix(kind as u64)));
            let backward =
                mix(self.vertex[source].color[phase].wrapping_add(mix((kind + 1) as u64)));
            self.vertex[source].summary[phase].insert(forward);
            self.vertex[target].summary[phase].insert(backward);
            self.mark(source, phase);
            self.mark(target, phase);
        }
    }

    pub fn disconnect(&mut self, source: usize, position: usize) {
        let (kind, target) = self.vertex[source].edge.swap_remove(position);
        let position = self.vertex[target]
            .edge
            .iter()
            .position(|&edge| edge == (kind ^ 1, source))
            .unwrap();
        self.vertex[target].edge.swap_remove(position);
        self.edge -= 2;
        for phase in 0..4 {
            let forward = mix(self.vertex[target].color[phase].wrapping_add(mix(kind as u64)));
            let backward =
                mix(self.vertex[source].color[phase].wrapping_add(mix((kind ^ 1) as u64)));
            self.vertex[source].summary[phase].remove(forward);
            self.vertex[target].summary[phase].remove(backward);
            self.mark(source, phase);
            self.mark(target, phase);
        }
    }

    pub fn remove(&mut self, vertex: usize) {
        while !self.vertex[vertex].edge.is_empty() {
            self.disconnect(vertex, self.vertex[vertex].edge.len() - 1);
        }
        self.summary.remove(self.vertex[vertex].color[4]);
        self.vertex[vertex].active = false;
        self.vacant.push(vertex);
    }

    fn color(&mut self, vertex: usize, phase: usize, value: u64) {
        let previous = std::mem::replace(&mut self.vertex[vertex].color[phase], value);
        if previous == value {
            return;
        }
        if phase == 4 {
            self.summary.remove(previous);
            self.summary.insert(value);
            return;
        }
        self.mark(vertex, phase);
        for position in 0..self.vertex[vertex].edge.len() {
            let (kind, target) = self.vertex[vertex].edge[position];
            let offset = mix((kind ^ 1) as u64);
            self.vertex[target].summary[phase].remove(mix(previous.wrapping_add(offset)));
            self.vertex[target].summary[phase].insert(mix(value.wrapping_add(offset)));
            self.mark(target, phase);
        }
    }

    pub fn replace(&mut self, vertex: usize, value: Label) {
        self.color(vertex, 0, label(value));
    }

    fn rebuild(&mut self, phase: usize) {
        self.pending[phase].clear();
        for vertex in &mut self.vertex {
            vertex.dirty[phase] = false;
            if vertex.active {
                vertex.color[phase + 1] =
                    mix(vertex.color[phase]).wrapping_add(vertex.summary[phase].value());
            }
        }
        if phase == 3 {
            self.summary = Accumulator::default();
            for vertex in self.vertex.iter().filter(|vertex| vertex.active) {
                self.summary.insert(vertex.color[4]);
            }
            return;
        }
        for index in 0..self.vertex.len() {
            if !self.vertex[index].active {
                continue;
            }
            let mut summary = Accumulator::default();
            for &(kind, target) in &self.vertex[index].edge {
                summary.insert(mix(
                    self.vertex[target].color[phase + 1].wrapping_add(mix(kind as u64))
                ));
            }
            self.vertex[index].summary[phase + 1] = summary;
            self.mark(index, phase + 1);
        }
    }

    pub fn advance(&mut self) -> u64 {
        for phase in 0..4 {
            if self.pending[phase].len() * 2 > self.vertex.len() - self.vacant.len() {
                self.rebuild(phase);
                continue;
            }
            while let Some(vertex) = self.pending[phase].pop() {
                if !self.vertex[vertex].active
                    || !std::mem::take(&mut self.vertex[vertex].dirty[phase])
                {
                    continue;
                }
                let value = mix(self.vertex[vertex].color[phase])
                    .wrapping_add(self.vertex[vertex].summary[phase].value());
                self.color(vertex, phase + 1, value);
            }
        }
        self.summary.value()
    }

    pub fn retained(&self) -> usize {
        self.vertex.len() * 10
            + self.edge
            + self.vacant.len()
            + self.pending.iter().map(Vec::len).sum::<usize>()
    }
}

impl std::ops::Index<usize> for Network {
    type Output = [(u8, usize)];

    fn index(&self, vertex: usize) -> &Self::Output {
        &self.vertex[vertex].edge
    }
}
