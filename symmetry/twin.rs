use crate::graph::{Color, Edge, Graph};
use std::collections::HashMap;

pub struct Quotient {
    pub graph: Graph,
    pub class: Vec<Vec<u32>>,
}

pub fn reduce(graph: &Graph) -> Quotient {
    let mut index = HashMap::<(Color, &[Edge]), u32>::new();
    let mut class: Vec<Vec<u32>> = Vec::new();
    let mut image = Vec::with_capacity(graph.len());
    for vertex in 0..graph.atom {
        let key = (graph.color[vertex], graph.parent[vertex].as_slice());
        let next = class.len() as u32;
        let member = *index.entry(key).or_insert(next);
        if member == next {
            class.push(Vec::new());
        }
        class[member as usize].push(vertex as u32);
        image.push(member);
    }
    let shift = class.len() as u32;
    image.extend((graph.atom..graph.len()).map(|vertex| vertex as u32 - graph.atom as u32 + shift));
    let mut color = class
        .iter()
        .map(|member| match graph.color[member[0] as usize] {
            Color::Atom(pin, _) => Color::Atom(pin, member.len() as u32),
            color => color,
        })
        .collect::<Vec<_>>();
    color.extend_from_slice(&graph.color[graph.atom..]);
    let mut child = vec![Vec::new(); class.len()];
    child.extend(graph.child[graph.atom..].iter().map(|edge| {
        let mut edge = edge
            .iter()
            .map(|edge| Edge {
                vertex: image[edge.vertex as usize],
                ..*edge
            })
            .collect::<Vec<_>>();
        edge.sort_unstable();
        edge.dedup();
        edge
    }));
    Quotient {
        graph: Graph::link(color, child, class.len()),
        class,
    }
}
