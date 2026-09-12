use crate::program::Symbol;
use crate::state::State;
use std::collections::BTreeMap;

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
enum Label {
    World,
    Frame(usize, bool),
    Resource(Symbol),
}

pub struct Refinement {
    pub world: Vec<usize>,
    pub frame: Vec<usize>,
}

fn classify<Key: Ord + Clone>(value: &[Key]) -> Vec<usize> {
    let mut label = value
        .iter()
        .cloned()
        .map(|value| (value, 0))
        .collect::<BTreeMap<_, _>>();
    for (index, ordinal) in label.values_mut().enumerate() {
        *ordinal = index;
    }
    value.iter().map(|value| label[value]).collect()
}

fn connect(edge: &mut [Vec<(u8, usize)>], source: usize, target: usize, kind: u8) {
    edge[source].push((kind, target));
    edge[target].push((kind + 1, source));
}

impl Refinement {
    pub fn new(state: &State) -> Self {
        let retained = state.reachable();
        let mut label = vec![Label::World; state.world.len()];
        let mut frame = vec![0; state.frame.len()];
        for &index in &retained {
            frame[index] = label.len();
            label.push(Label::Frame(state.frame[index].scope, index == 0));
        }
        let mut resource = BTreeMap::new();
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
        let mut edge = vec![Vec::new(); label.len()];
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
        let mut capture = BTreeMap::new();
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
        let mut color = classify(&label);
        loop {
            let signature = edge
                .iter()
                .enumerate()
                .map(|(index, edge)| {
                    let mut adjacent = edge
                        .iter()
                        .map(|&(kind, target)| (kind, color[target]))
                        .collect::<Vec<_>>();
                    adjacent.sort();
                    (color[index], adjacent)
                })
                .collect::<Vec<_>>();
            let next = classify(&signature);
            if next == color {
                break;
            }
            color = next;
        }
        Self {
            world: color[..state.world.len()].to_vec(),
            frame: frame.into_iter().map(|index| color[index]).collect(),
        }
    }
}
