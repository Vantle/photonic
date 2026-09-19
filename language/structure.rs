use crate::incidence::Label;
use crate::link::Link;
use crate::state::{State, Token};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

struct World {
    value: Arc<crate::state::World>,
    vertex: usize,
}

struct Frame {
    value: Arc<crate::state::Frame>,
    vertex: usize,
}

#[derive(Default)]
pub(crate) struct Structure {
    graph: crate::propagation::Network,
    world: HashMap<usize, World>,
    frame: Vec<Option<Frame>>,
    resource: HashMap<usize, usize>,
}

impl Structure {
    fn token(&mut self, token: &Token) -> usize {
        let vertex = if let Some(&vertex) = self.resource.get(&token.id) {
            self.graph.replace(vertex, Label::Resource(token.value));
            vertex
        } else {
            let vertex = self.graph.insert(Label::Resource(token.value));
            self.resource.insert(token.id, vertex);
            vertex
        };
        let capture = token
            .capture
            .map(|frame| self.frame[frame].as_ref().unwrap().vertex);
        let previous = self.graph[vertex]
            .iter()
            .position(|&(kind, _)| kind == Link::Capture);
        if previous.map(|position| self.graph[vertex][position].1) == capture {
            return vertex;
        }
        if let Some(position) = previous {
            self.graph.disconnect(vertex, position);
        }
        if let Some(capture) = capture {
            self.graph.connect(vertex, capture, Link::Capture);
        }
        vertex
    }

    fn retain(&mut self, current: &HashSet<usize>) {
        let removal = self
            .world
            .keys()
            .copied()
            .filter(|key| !current.contains(key))
            .collect::<Vec<_>>();
        for key in removal {
            let world = self.world.remove(&key).unwrap();
            self.graph.remove(world.vertex);
        }
    }

    fn prepare(&mut self, index: usize, value: &Arc<crate::state::Frame>) -> bool {
        let vertex = if let Some(frame) = &self.frame[index] {
            if Arc::ptr_eq(&frame.value, value) {
                return false;
            }
            let vertex = frame.vertex;
            while let Some(position) = self.graph[vertex]
                .iter()
                .position(|&(kind, _)| matches!(kind, Link::Parent | Link::Lexical | Link::Held))
            {
                self.graph.disconnect(vertex, position);
            }
            self.graph
                .replace(vertex, Label::Frame(value.scope, index == 0));
            vertex
        } else {
            self.graph.insert(Label::Frame(value.scope, index == 0))
        };
        self.frame[index] = Some(Frame {
            value: value.clone(),
            vertex,
        });
        true
    }

    fn frame(&mut self, index: usize) {
        let frame = self.frame[index].as_ref().unwrap();
        let vertex = frame.vertex;
        let value = frame.value.clone();
        if let Some(parent) = value.parent {
            self.graph.connect(
                vertex,
                self.frame[parent].as_ref().unwrap().vertex,
                Link::Parent,
            );
        }
        if let Some(lexical) = value.lexical {
            self.graph.connect(
                vertex,
                self.frame[lexical].as_ref().unwrap().vertex,
                Link::Lexical,
            );
        }
        for token in &value.held {
            let resource = self.token(token);
            self.graph.connect(vertex, resource, Link::Held);
        }
    }

    fn world(&mut self, value: &Arc<crate::state::World>) {
        let key = Arc::as_ptr(value) as usize;
        if self
            .world
            .get(&key)
            .is_some_and(|previous| Arc::ptr_eq(&previous.value, value))
        {
            return;
        }
        let vertex = self.graph.insert(Label::World);
        self.graph.connect(
            vertex,
            self.frame[value.frame].as_ref().unwrap().vertex,
            Link::Context,
        );
        for token in &value.particle {
            let resource = self.token(token);
            self.graph.connect(vertex, resource, Link::Member);
        }
        self.world.insert(
            key,
            World {
                value: value.clone(),
                vertex,
            },
        );
    }

    fn reclaim(&mut self, reachable: &[usize]) {
        for index in 0..self.frame.len() {
            if reachable.binary_search(&index).is_err()
                && let Some(frame) = self.frame[index].take()
            {
                self.graph.remove(frame.vertex);
            }
        }
        let removal = self
            .resource
            .iter()
            .filter_map(|(&id, &vertex)| {
                (!self.graph[vertex]
                    .iter()
                    .any(|&(kind, _)| matches!(kind, Link::Particle | Link::Holder)))
                .then_some(id)
            })
            .collect::<Vec<_>>();
        for id in removal {
            let vertex = self.resource.remove(&id).unwrap();
            self.graph.remove(vertex);
        }
    }

    pub fn advance(&mut self, state: &State) -> u64 {
        let current = state
            .world
            .iter()
            .map(|world| Arc::as_ptr(world) as usize)
            .collect::<HashSet<_>>();
        if current.len() != state.world.len() {
            return crate::fingerprint::signature(state);
        }
        self.retain(&current);
        let reachable = state.reachable();
        self.frame
            .resize_with(self.frame.len().max(state.frame.len()), || None);
        let change = reachable
            .iter()
            .copied()
            .filter(|&index| self.prepare(index, &state.frame[index]))
            .collect::<Vec<_>>();
        for index in change {
            self.frame(index);
        }
        for world in &state.world {
            self.world(world);
        }
        self.reclaim(&reachable);
        self.graph.advance()
    }

    pub fn retained(&self) -> usize {
        self.graph.retained() + self.world.len() + self.frame.len() + self.resource.len()
    }
}

#[cfg(test)]
#[path = "test/structure.rs"]
mod test;
