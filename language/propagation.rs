use crate::accumulator::Accumulator;
use crate::color;
use crate::incidence::Label;
use crate::link::Link;
use hashing::mix;

pub(crate) const DEPTH: usize = 4;

#[derive(Default)]
struct Vertex {
    active: bool,
    edge: Vec<(Link, usize)>,
    color: [u64; DEPTH + 1],
    summary: [Accumulator; DEPTH],
    dirty: [bool; DEPTH],
}

#[derive(Default)]
pub(crate) struct Network {
    vertex: Vec<Vertex>,
    vacant: Vec<usize>,
    pending: [Vec<usize>; DEPTH],
    summary: Accumulator,
    edge: usize,
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
            color: std::array::from_fn(|phase| if phase == 0 { color::label(&value) } else { 0 }),
            ..Vertex::default()
        };
        self.summary.insert(0);
        for phase in 0..DEPTH {
            self.mark(index, phase);
        }
        index
    }

    pub fn connect(&mut self, source: usize, target: usize, kind: Link) {
        self.vertex[source].edge.push((kind, target));
        self.vertex[target].edge.push((kind.reverse(), source));
        self.edge += 2;
        for phase in 0..DEPTH {
            let forward = color::edge(kind, self.vertex[target].color[phase]);
            let backward = color::edge(kind.reverse(), self.vertex[source].color[phase]);
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
            .position(|&edge| edge == (kind.reverse(), source))
            .unwrap();
        self.vertex[target].edge.swap_remove(position);
        self.edge -= 2;
        for phase in 0..DEPTH {
            let forward = color::edge(kind, self.vertex[target].color[phase]);
            let backward = color::edge(kind.reverse(), self.vertex[source].color[phase]);
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
        self.summary.remove(self.vertex[vertex].color[DEPTH]);
        self.vertex[vertex].active = false;
        self.vacant.push(vertex);
    }

    fn color(&mut self, vertex: usize, phase: usize, value: u64) {
        let previous = std::mem::replace(&mut self.vertex[vertex].color[phase], value);
        if previous == value {
            return;
        }
        if phase == DEPTH {
            self.summary.remove(previous);
            self.summary.insert(value);
            return;
        }
        self.mark(vertex, phase);
        for position in 0..self.vertex[vertex].edge.len() {
            let (kind, target) = self.vertex[vertex].edge[position];
            let kind = kind.reverse();
            self.vertex[target].summary[phase].remove(color::edge(kind, previous));
            self.vertex[target].summary[phase].insert(color::edge(kind, value));
            self.mark(target, phase);
        }
    }

    pub fn replace(&mut self, vertex: usize, value: Label) {
        self.color(vertex, 0, color::label(&value));
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
        if phase + 1 == DEPTH {
            self.summary = Accumulator::default();
            for vertex in self.vertex.iter().filter(|vertex| vertex.active) {
                self.summary.insert(vertex.color[DEPTH]);
            }
            return;
        }
        for index in 0..self.vertex.len() {
            if !self.vertex[index].active {
                continue;
            }
            let mut summary = Accumulator::default();
            for &(kind, target) in &self.vertex[index].edge {
                summary.insert(color::edge(kind, self.vertex[target].color[phase + 1]));
            }
            self.vertex[index].summary[phase + 1] = summary;
            self.mark(index, phase + 1);
        }
    }

    pub fn advance(&mut self) -> u64 {
        for phase in 0..DEPTH {
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
    type Output = [(Link, usize)];

    fn index(&self, vertex: usize) -> &Self::Output {
        &self.vertex[vertex].edge
    }
}
