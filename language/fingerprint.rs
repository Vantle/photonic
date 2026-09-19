use crate::basis::Set;
use crate::program::Symbol;
use crate::state::{State, Token};
use std::sync::Arc;

fn mix(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

fn symbol(value: Symbol) -> u64 {
    match value {
        Symbol::Atom(index) => mix((index as u64).wrapping_mul(2)),
        Symbol::Rule(index) => mix((index as u64).wrapping_mul(2).wrapping_add(1)),
    }
}

fn collection(value: impl IntoIterator<Item = u64>) -> u64 {
    let mut sum = 0u64;
    let mut square = 0u64;
    let mut count = 0u64;
    for value in value {
        sum = sum.wrapping_add(value);
        square = square.wrapping_add(value.wrapping_mul(value));
        count += 1;
    }
    mix(sum)
        .wrapping_add(mix(square).rotate_left(21))
        .wrapping_add(mix(count).rotate_left(42))
}

fn particle(value: &[Token], frame: &[u64]) -> u64 {
    collection(value.iter().map(|token| {
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
    retained: usize,
}

impl Index {
    pub(crate) fn new(state: Arc<State>) -> Self {
        Self::construct(state, None, &Set::default())
    }

    pub(crate) fn advance(&self, state: Arc<State>, removed: &Set<usize>) -> Self {
        Self::construct(state, Some(self), removed)
    }

    fn construct(state: Arc<State>, previous: Option<&Self>, removed: &Set<usize>) -> Self {
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
                mix(value.scope as u64).wrapping_add(collection(
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
        let value = mix(collection(world.iter().map(|world| world.value))).wrapping_add(
            collection(state.reachable().into_iter().map(|index| frame[2][index])).rotate_left(31),
        );
        let retained = frame.iter().map(Vec::len).sum::<usize>()
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
        }
    }

    pub(crate) fn retained(&self) -> usize {
        self.retained
    }
}

pub(crate) fn state(state: &State) -> u64 {
    Index::new(Arc::new(state.clone())).value
}
