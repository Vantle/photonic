use crate::program::Symbol;
use crate::state::State;
use std::collections::HashMap;

#[derive(Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum Label {
    World,
    Frame(usize, bool),
    Resource(Symbol),
}

pub(crate) struct Incidence {
    pub label: Vec<Label>,
    pub frame: Vec<usize>,
    pub edge: crate::graph::Graph,
}

fn connect(edge: &mut Vec<(usize, usize, u8)>, source: usize, target: usize, kind: u8) {
    edge.push((source, target, kind));
    edge.push((target, source, kind + 1));
}

impl Incidence {
    pub fn new(state: &State) -> Self {
        let retained = state.reachable();
        let mut label = vec![Label::World; state.world.len()];
        let mut frame = vec![0; state.frame.len()];
        for &index in &retained {
            frame[index] = label.len();
            label.push(Label::Frame(state.frame[index].scope, index == 0));
        }
        let mut resource = HashMap::new();
        for token in state
            .world
            .iter()
            .flat_map(|world| &world.particle)
            .chain(retained.iter().flat_map(|&index| &state.frame[index].held))
        {
            resource.entry(token.id).or_insert_with(|| {
                let index = label.len();
                label.push(Label::Resource(token.value));
                index
            });
        }
        let mut edge = Vec::new();
        for (index, world) in state.world.iter().enumerate() {
            connect(&mut edge, index, frame[world.frame], 0);
            for token in &world.particle {
                connect(&mut edge, index, resource[&token.id], 2);
            }
        }
        for &index in &retained {
            let value = &state.frame[index];
            if let Some(parent) = value.parent {
                connect(&mut edge, frame[index], frame[parent], 4);
            }
            if let Some(lexical) = value.lexical {
                connect(&mut edge, frame[index], frame[lexical], 6);
            }
            for token in &value.held {
                connect(&mut edge, frame[index], resource[&token.id], 8);
            }
        }
        let mut capture = HashMap::new();
        for token in state
            .world
            .iter()
            .flat_map(|world| &world.particle)
            .chain(retained.iter().flat_map(|&index| &state.frame[index].held))
        {
            if let Some(frame) = token.capture {
                capture.insert(token.id, frame);
            }
        }
        for (id, captured) in capture {
            connect(&mut edge, resource[&id], frame[captured], 10);
        }
        let edge = crate::graph::Graph::new(label.len(), edge);
        Self { label, frame, edge }
    }
}
