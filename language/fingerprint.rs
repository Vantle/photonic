use crate::accumulator::Accumulator;
use crate::hashing::mix;
use crate::program::Symbol;
use crate::state::{State, Token};
use std::sync::Arc;

mod context;
mod dependency;
mod population;

fn symbol(value: Symbol) -> u64 {
    match value {
        Symbol::Atom(index) => mix((index as u64).wrapping_mul(2)),
        Symbol::Rule(index) => mix((index as u64).wrapping_mul(2).wrapping_add(1)),
    }
}

fn particle<'token>(value: impl IntoIterator<Item = &'token Token>, frame: &[u64]) -> u64 {
    Accumulator::collect(value.into_iter().map(|token| {
        mix(symbol(token.value).wrapping_add(
            token
                .capture
                .map_or(0, |index| frame[index])
                .rotate_left(29),
        ))
    }))
}

struct World {
    value: u64,
    dependency: Vec<usize>,
}

impl World {
    fn new(world: &crate::state::World, frame: &[u64]) -> Self {
        let value =
            mix(frame[world.frame].wrapping_add(particle(&world.particle, frame).rotate_left(17)));
        let mut dependency = world
            .particle
            .iter()
            .filter_map(|token| token.capture)
            .collect::<Vec<_>>();
        dependency.push(world.frame);
        dependency.sort_unstable();
        dependency.dedup();
        Self { value, dependency }
    }
}

pub(crate) struct Index {
    frame: context::Index,
    world: crate::sequence::List<Arc<World>>,
    aggregate: Accumulator,
    context: u64,
    dependency: usize,
    pub value: u64,
    pub layout: crate::layout::Layout,
    retained: usize,
}

impl Index {
    pub(crate) fn new(state: Arc<State>) -> Self {
        let layout = crate::layout::Layout::new(&state);
        let frame = context::Index::new(&state);
        let world = state
            .world
            .iter()
            .map(|world| Arc::new(World::new(world, frame.color())))
            .collect::<crate::sequence::List<_>>();
        let mut aggregate = Accumulator::default();
        for world in &world {
            aggregate.insert(world.value);
        }
        let context =
            Accumulator::collect(layout.reach.frame.iter().map(|&index| frame.color()[index]));
        let dependency = world
            .iter()
            .map(|world| world.dependency.len())
            .sum::<usize>();
        let retained = layout.reach.retained() + 1 + frame.retained() + world.len() + dependency;
        Self {
            value: mix(aggregate.value()).wrapping_add(context.rotate_left(31)),
            frame,
            world,
            aggregate,
            context,
            dependency,
            layout,
            retained,
        }
    }

    pub(crate) fn advance(
        &self,
        state: Arc<State>,
        change: &crate::change::Change,
        layout: crate::layout::Layout,
    ) -> Self {
        #[cfg(feature = "measurement")]
        let _measurement = crate::measurement::profile::Scope::new(
            crate::measurement::profile::Phase::Fingerprint,
        );
        let (frame, changed) = self.frame.advance(&state, &change.frame);
        let mut world = self.world.clone();
        let mut aggregate = self.aggregate;
        let mut dependency = self.dependency;
        for &index in change.world.iter().rev() {
            let previous = world.remove(index);
            aggregate.remove(previous.value);
            dependency -= previous.dependency.len();
        }
        if !changed.is_empty() {
            for position in 0..world.len() {
                let previous = &world[position];
                if !previous
                    .dependency
                    .iter()
                    .any(|index| changed.binary_search(index).is_ok())
                {
                    continue;
                }
                let current = Arc::new(World::new(&state.world[position], frame.color()));
                aggregate.remove(previous.value);
                aggregate.insert(current.value);
                dependency = dependency - previous.dependency.len() + current.dependency.len();
                world[position] = current;
            }
        }
        for value in state.world.range(change.insertion.clone()) {
            let value = Arc::new(World::new(value, frame.color()));
            aggregate.insert(value.value);
            dependency += value.dependency.len();
            world.push(value);
        }
        let context =
            if changed.is_empty() && Arc::ptr_eq(&layout.reach.frame, &self.layout.reach.frame) {
                self.context
            } else {
                Accumulator::collect(layout.reach.frame.iter().map(|&index| frame.color()[index]))
            };
        let value = mix(aggregate.value()).wrapping_add(context.rotate_left(31));
        let retained = layout.reach.retained() + 1 + frame.retained() + world.len() + dependency;
        Self {
            frame,
            world,
            aggregate,
            context,
            dependency,
            value,
            layout,
            retained,
        }
    }

    pub(crate) fn evict(&mut self) -> usize {
        let released = self.layout.reach.evict() + self.frame.evict();
        self.retained -= released;
        released
    }

    pub(crate) fn retained(&self) -> usize {
        self.retained
    }
}

pub(crate) fn state(state: &State) -> u64 {
    Index::new(Arc::new(state.clone())).value
}

pub(crate) fn signature(state: &State) -> u64 {
    refine(state, 4)
}

pub(crate) fn resolution(state: &State) -> u64 {
    refine(state, 8)
}

fn refine(state: &State, depth: usize) -> u64 {
    let incidence = crate::incidence::Incidence::new(state);
    let mut color = incidence
        .label
        .iter()
        .map(crate::hashing::value)
        .collect::<Vec<_>>();
    for _ in 0..depth {
        color = incidence
            .edge
            .iter()
            .enumerate()
            .map(|(index, edge)| {
                mix(color[index]).wrapping_add(Accumulator::collect(
                    edge.iter()
                        .map(|&(kind, target)| crate::hashing::edge(kind, color[target])),
                ))
            })
            .collect();
    }
    Accumulator::collect(color)
}

#[cfg(test)]
#[path = "test/fingerprint.rs"]
mod test;
