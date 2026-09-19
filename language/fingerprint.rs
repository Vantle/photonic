use crate::accumulator::Accumulator;
use crate::basis::Set;
use crate::hashing::mix;
use crate::program::Symbol;
use crate::state::{State, Token};
use std::sync::Arc;

fn symbol(value: Symbol) -> u64 {
    match value {
        Symbol::Atom(index) => mix((index as u64).wrapping_mul(2)),
        Symbol::Rule(index) => mix((index as u64).wrapping_mul(2).wrapping_add(1)),
    }
}

fn particle(value: &[Token], frame: &[u64]) -> u64 {
    Accumulator::collect(value.iter().map(|token| {
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
    state: Arc<State>,
    frame: [Vec<u64>; 3],
    world: Vec<Arc<World>>,
    pub value: u64,
    pub layout: crate::layout::Layout,
    retained: usize,
}

impl Index {
    pub(crate) fn new(state: Arc<State>) -> Self {
        let reachable = state.reachable();
        Self::construct(state, None, &Set::default(), reachable)
    }

    pub(crate) fn advance(
        &self,
        state: Arc<State>,
        removed: &Set<usize>,
        reachable: Vec<usize>,
    ) -> Self {
        Self::construct(state, Some(self), removed, reachable)
    }

    fn construct(
        state: Arc<State>,
        previous: Option<&Self>,
        removed: &Set<usize>,
        reachable: Vec<usize>,
    ) -> Self {
        let stable = state
            .frame
            .iter()
            .enumerate()
            .map(|(index, frame)| {
                previous.is_some_and(|previous| {
                    previous
                        .state
                        .frame
                        .get(index)
                        .is_some_and(|old| Arc::ptr_eq(old, frame))
                })
            })
            .collect::<Vec<_>>();
        let mut frame = [
            Vec::with_capacity(state.frame.len()),
            Vec::with_capacity(state.frame.len()),
            Vec::with_capacity(state.frame.len()),
        ];
        for (index, value) in state.frame.iter().enumerate() {
            let hash = if stable[index] {
                previous.unwrap().frame[0][index]
            } else {
                mix(value.scope as u64).wrapping_add(Accumulator::collect(
                    value.held.iter().map(|token| symbol(token.value)),
                ))
            };
            frame[0].push(hash);
        }
        for phase in 1..3 {
            for (index, value) in state.frame.iter().enumerate() {
                let unchanged = stable[index]
                    && value
                        .parent
                        .into_iter()
                        .chain(value.lexical)
                        .chain(value.held.iter().filter_map(|token| token.capture))
                        .all(|index| {
                            previous.unwrap().frame[phase - 1][index] == frame[phase - 1][index]
                        });
                let hash = if unchanged {
                    previous.unwrap().frame[phase][index]
                } else {
                    mix(value.scope as u64)
                        .wrapping_add(particle(&value.held, &frame[phase - 1]))
                        .wrapping_add(
                            value
                                .parent
                                .map_or(0, |index| frame[phase - 1][index])
                                .rotate_left(13),
                        )
                        .wrapping_add(
                            value
                                .lexical
                                .map_or(0, |index| frame[phase - 1][index])
                                .rotate_left(37),
                        )
                };
                frame[phase].push(hash);
            }
        }
        let mut world = Vec::with_capacity(state.world.len());
        if let Some(previous) = previous {
            for (index, old) in previous.world.iter().enumerate() {
                if removed.contains(&index) {
                    continue;
                }
                let value = if old
                    .dependency
                    .iter()
                    .all(|&index| previous.frame[2][index] == frame[2][index])
                {
                    old.clone()
                } else {
                    Arc::new(World::new(&state.world[world.len()], &frame[2]))
                };
                world.push(value);
            }
        }
        for value in &state.world[world.len()..] {
            world.push(Arc::new(World::new(value, &frame[2])));
        }
        let value = mix(Accumulator::collect(world.iter().map(|world| world.value))).wrapping_add(
            Accumulator::collect(reachable.iter().map(|&index| frame[2][index])).rotate_left(31),
        );
        let layout = crate::layout::Layout::new(&state, reachable);
        let retained = layout.reachable.len()
            + 1
            + frame.iter().map(Vec::len).sum::<usize>()
            + world.len()
            + world
                .iter()
                .map(|world| world.dependency.len())
                .sum::<usize>();
        Self {
            state,
            frame,
            world,
            value,
            retained,
            layout,
        }
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
