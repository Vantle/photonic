use super::composition::Composition;
use super::{Flow, Place};
use crate::basis::Set;
use crate::hashing::Builder;
use crate::profile;
use std::collections::HashMap;

pub(super) struct Template {
    resource: Vec<(Set<Place>, Vec<Place>)>,
    context: Vec<(Set<usize>, Vec<usize>)>,
    frame: Vec<Option<usize>>,
    width: usize,
    retained: usize,
}

fn group<Value: Clone + Eq + std::hash::Hash, Position>(
    source: impl Iterator<Item = (Position, Set<Value>)>,
) -> Vec<(Set<Value>, Vec<Position>)> {
    let mut identity = HashMap::<_, _, Builder>::default();
    let mut result: Vec<(Set<Value>, Vec<Position>)> = Vec::new();
    for (position, value) in source {
        let next = result.len();
        let index = *identity.entry(value.clone()).or_insert(next);
        if index == next {
            result.push((value, Vec::new()));
        }
        result[index].1.push(position);
    }
    result
}

impl Template {
    pub fn new(event: &Flow) -> Self {
        let resource = group(
            event
                .resource
                .iter()
                .map(|(&place, source)| (place, source.clone())),
        );
        let context = group(event.context.iter().cloned().enumerate());
        let retained = event.frame.len()
            + 1
            + resource
                .iter()
                .map(|(source, destination)| source.len() + destination.len() + 2)
                .sum::<usize>()
            + context
                .iter()
                .map(|(source, destination)| source.len() + destination.len() + 2)
                .sum::<usize>();
        Self {
            resource,
            context,
            frame: event.frame.clone(),
            width: event.context.len(),
            retained,
        }
    }

    pub fn compose(&self, parent: &Flow, composition: &mut Composition) -> Flow {
        let _scope = profile::Scope::new(profile::Phase::Composition);
        let resource = self
            .resource
            .iter()
            .flat_map(|(source, destination)| {
                let value = composition.resource(parent, source);
                destination.iter().map(move |&place| (place, value.clone()))
            })
            .collect();
        let mut context = vec![Set::default(); self.width];
        for (source, destination) in &self.context {
            let value = composition.context(parent, source);
            for &position in destination {
                context[position] = value.clone();
            }
        }
        Flow {
            resource,
            context,
            frame: self
                .frame
                .iter()
                .map(|index| index.and_then(|index| parent.frame[index]))
                .collect(),
        }
    }

    pub fn retained(&self) -> usize {
        self.retained
    }
}
