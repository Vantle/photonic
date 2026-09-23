use crate::basis::Set;
use crate::profile;
use crate::sequence::List;
use crate::state::State;
use imbl::OrdMap;
use std::collections::BTreeMap;
use std::sync::Arc;

fn reference(
    frame: &crate::state::Frame,
    population: Option<&super::population::Index>,
) -> Set<usize> {
    let source = frame
        .parent
        .into_iter()
        .chain(frame.lexical)
        .chain(frame.held.iter().filter_map(|token| token.capture));
    match population {
        Some(population) => population.reference(source),
        None => source
            .chain(frame.particle.iter().filter_map(|token| token.capture))
            .collect(),
    }
}

#[derive(Clone, Default)]
pub(super) struct Index {
    incoming: List<Set<usize>>,
    outgoing: List<Set<usize>>,
    population: OrdMap<usize, Arc<super::population::Index>>,
    edge: usize,
    retained: usize,
}

#[derive(Default)]
struct Change {
    removal: Vec<usize>,
    insertion: Vec<usize>,
}

impl Index {
    pub fn advance(&self, state: &State, changed: &[usize]) -> Self {
        let _scope = profile::Scope::new(profile::Phase::Dependency);
        let mut index = self.clone();
        while index.incoming.len() < state.frame.len() {
            index.incoming.push(Set::default());
            index.outgoing.push(Set::default());
        }
        let mut change = BTreeMap::<usize, Change>::new();
        for &source in changed {
            let previous = self.population.get(&source).map(Arc::as_ref);
            let population = state
                .frame
                .get(source)
                .filter(|frame| !frame.particle.is_empty())
                .map(|frame| {
                    Arc::new(match previous {
                        Some(previous) => previous.advance(&frame.particle),
                        None => super::population::Index::default().advance(&frame.particle),
                    })
                });
            index.retained = index.retained
                - previous.map_or(0, super::population::Index::retained)
                + population
                    .as_ref()
                    .map_or(0, |population| population.retained());
            let current = state
                .frame
                .get(source)
                .map(|frame| reference(frame, population.as_deref()))
                .unwrap_or_default();
            if let Some(population) = population {
                index.population.insert(source, population);
            } else {
                index.population.remove(&source);
            }
            let previous = self.outgoing.get(source).cloned().unwrap_or_default();
            if previous == current {
                continue;
            }
            for &target in previous.iter().filter(|target| !current.contains(target)) {
                change.entry(target).or_default().removal.push(source);
            }
            for &target in current.iter().filter(|target| !previous.contains(target)) {
                change.entry(target).or_default().insertion.push(source);
            }
            index.edge = index.edge - previous.len() + current.len();
            if source < state.frame.len() {
                index.outgoing[source] = current;
            }
        }
        for (target, change) in change {
            index.incoming[target] = index.incoming[target]
                .iter()
                .copied()
                .filter(|source| change.removal.binary_search(source).is_err())
                .chain(change.insertion)
                .collect();
        }
        index.incoming.truncate(state.frame.len());
        index.outgoing.truncate(state.frame.len());
        index
    }

    pub fn dependent(&self, frame: usize) -> std::slice::Iter<'_, usize> {
        self.incoming[frame].iter()
    }

    pub fn retained(&self) -> usize {
        self.incoming.len() + self.outgoing.len() + self.edge * 2 + self.retained
    }
}
