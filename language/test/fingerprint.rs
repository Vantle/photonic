use super::Index;
use crate::accumulator::Accumulator;
use crate::change::Change;
use crate::hashing::mix;
use crate::program::Symbol;
use crate::state::{Frame, State, Token, World};
use std::sync::Arc;

fn symbol(value: Symbol) -> u64 {
    match value {
        Symbol::Atom(index) => mix((index as u64).wrapping_mul(2)),
        Symbol::Rule(index) => mix((index as u64).wrapping_mul(2).wrapping_add(1)),
    }
}

fn reference(state: &State) -> u64 {
    let mut color = state
        .frame
        .iter()
        .map(|frame| {
            mix(frame.scope as u64)
                .wrapping_add(Accumulator::collect(
                    frame.held.iter().map(|token| symbol(token.value)),
                ))
                .wrapping_add(
                    Accumulator::collect(frame.particle.iter().map(|token| symbol(token.value)))
                        .rotate_left(7),
                )
        })
        .collect::<Vec<_>>();
    for _ in 0..2 {
        let token = |token: &Token| {
            mix(symbol(token.value).wrapping_add(
                token
                    .capture
                    .map_or(0, |frame| color[frame])
                    .rotate_left(29),
            ))
        };
        color = state
            .frame
            .iter()
            .map(|frame| {
                mix(frame.scope as u64)
                    .wrapping_add(Accumulator::collect(frame.held.iter().map(token)))
                    .wrapping_add(
                        Accumulator::collect(frame.particle.iter().map(token)).rotate_left(7),
                    )
                    .wrapping_add(frame.parent.map_or(0, |frame| color[frame]).rotate_left(13))
                    .wrapping_add(
                        frame
                            .lexical
                            .map_or(0, |frame| color[frame])
                            .rotate_left(37),
                    )
            })
            .collect();
    }
    let world = Accumulator::collect(state.world.iter().map(|world| {
        let particle = Accumulator::collect(world.particle.iter().map(|token| {
            mix(symbol(token.value).wrapping_add(
                token
                    .capture
                    .map_or(0, |frame| color[frame])
                    .rotate_left(29),
            ))
        }));
        mix(color[world.frame].wrapping_add(particle.rotate_left(17)))
    }));
    mix(world).wrapping_add(
        Accumulator::collect(state.reachable().into_iter().map(|frame| color[frame]))
            .rotate_left(31),
    )
}

fn state(width: usize) -> State {
    State {
        frame: (0..width)
            .map(|frame| {
                Arc::new(Frame {
                    scope: frame % 4,
                    parent: (frame > 0).then_some(0),
                    lexical: (frame > 0).then_some(frame / 2),
                    particle: (0..5)
                        .map(|position| Token {
                            id: frame * 6 + position,
                            value: Symbol::Rule(position % 3),
                            capture: Some((frame + position) % width),
                        })
                        .collect(),
                    held: vec![Token {
                        id: frame * 6 + 5,
                        value: Symbol::Rule(3),
                        capture: Some((frame + 1) % width),
                    }],
                })
            })
            .collect(),
        world: (0..width * 2)
            .map(|world| {
                Arc::new(World {
                    frame: world % width,
                    particle: vec![Token {
                        id: width * 6 + world,
                        value: Symbol::Rule(world % 4),
                        capture: Some((world + 3) % width),
                    }],
                })
            })
            .collect(),
    }
}

fn verify(index: &Index, state: &State) {
    let fresh = Index::new(Arc::new(state.clone()));
    assert_eq!(index.value, reference(state));
    assert_eq!(index.value, fresh.value);
    assert_eq!(index.frame.color(), fresh.frame.color());
    let capture = state
        .frame
        .iter()
        .map(|frame| {
            frame
                .particle
                .iter()
                .filter_map(|token| token.capture)
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        })
        .sum::<usize>();
    assert!(index.retained() >= fresh.retained());
    assert!(index.retained() <= fresh.retained() + capture);
}

#[test]
fn dependency() {
    for width in [1, 2, 8, 65, 257] {
        let original = state(width);
        let root = Index::new(Arc::new(original.clone()));
        let mut state = original.clone();
        let mut index = Index::new(Arc::new(state.clone()));
        for iteration in 0..48 {
            let position = iteration * 17 % width;
            let frame = Arc::make_mut(&mut state.frame[position]);
            match iteration % 6 {
                0 => frame.scope += 1,
                1 => frame.particle.retain(|token| token.id % 2 == 1),
                2 => frame.held[0].capture = Some((position + 5) % width),
                3 => frame.lexical = (position > 0).then(|| position - 1),
                4 => frame.particle = original.frame[position].particle.clone(),
                _ => frame.held[0].value = Symbol::Atom(iteration),
            }
            let change = Change {
                world: [0, state.world.len() - 1].into_iter().collect(),
                insertion: state.world.len() - 2..state.world.len(),
                frame: vec![position],
            };
            for &world in change.world.iter().rev() {
                state.world.remove(world);
            }
            state
                .world
                .push(original.world[iteration % original.world.len()].clone());
            state
                .world
                .push(original.world[(iteration + 1) % original.world.len()].clone());
            index = index.advance(
                Arc::new(state.clone()),
                &change,
                crate::layout::Layout::new(&state),
            );
            verify(&index, &state);
            verify(&root, &original);
        }
    }
}

#[test]
fn retirement() {
    let program = crate::program::Program::new(&frontend::lowering::parse("A, [A] B").unwrap());
    let original = State::initial(&program);
    let mut index = Index::new(Arc::new(original.clone()));
    for iteration in 0..16 {
        let mut state = original.clone();
        state.frame.push(Arc::new(Frame {
            scope: iteration,
            parent: Some(0),
            lexical: Some(0),
            particle: vec![Token {
                id: 100 + iteration,
                value: Symbol::Rule(iteration),
                capture: Some(1),
            }]
            .into(),
            held: Vec::new(),
        }));
        state.world.push(Arc::new(World {
            frame: 1,
            particle: Vec::new(),
        }));
        let change = Change {
            world: Default::default(),
            insertion: 1..2,
            frame: vec![1],
        };
        index = index.advance(
            Arc::new(state.clone()),
            &change,
            crate::layout::Layout::new(&state),
        );
        verify(&index, &state);
        let change = Change {
            world: [1].into_iter().collect(),
            insertion: 1..1,
            frame: vec![1],
        };
        index = index.advance(
            Arc::new(original.clone()),
            &change,
            crate::layout::Layout::new(&original),
        );
        verify(&index, &original);
    }
}

#[test]
fn locality() {
    let mut state = state(8);
    for frame in 0..8 {
        let value = Arc::make_mut(&mut state.frame[frame]);
        value.particle.clear();
        value.held.clear();
        value.parent = None;
        value.lexical = None;
    }
    for world in 0..state.world.len() {
        Arc::make_mut(&mut state.world[world]).particle.clear();
    }
    let index = Index::new(Arc::new(state.clone()));
    Arc::make_mut(&mut state.frame[3]).scope += 1;
    let change = Change {
        world: Default::default(),
        insertion: state.world.len()..state.world.len(),
        frame: vec![3],
    };
    let advanced = index.advance(
        Arc::new(state.clone()),
        &change,
        crate::layout::Layout::new(&state),
    );
    verify(&advanced, &state);
    for world in 0..state.world.len() {
        assert_eq!(
            Arc::ptr_eq(&index.world[world], &advanced.world[world]),
            state.world[world].frame != 3
        );
    }
}

#[test]
fn adjacency() {
    for width in [2, 65, 257] {
        let mut state = state(width);
        let changed = (0..width).collect::<Vec<_>>();
        let mut index = super::dependency::Index::default();
        for iteration in 0..12 {
            for position in 0..width {
                let frame = Arc::make_mut(&mut state.frame[position]);
                frame.held[0].capture = Some(iteration % width);
                if iteration % 2 == 0 {
                    frame.particle.clear();
                } else {
                    frame.particle = (0..3)
                        .map(|offset| Token {
                            id: position * 3 + offset,
                            value: Symbol::Rule(offset),
                            capture: Some(iteration % width),
                        })
                        .collect();
                }
            }
            index = index.advance(&state, &changed);
            let mut count = 0;
            for target in 0..width {
                let expected = state
                    .frame
                    .iter()
                    .enumerate()
                    .filter_map(|(source, frame)| {
                        frame
                            .parent
                            .into_iter()
                            .chain(frame.lexical)
                            .chain(frame.token().filter_map(|token| token.capture))
                            .any(|frame| frame == target)
                            .then_some(source)
                    })
                    .collect::<Vec<_>>();
                count += expected.len();
                assert_eq!(
                    index.dependent(target).copied().collect::<Vec<_>>(),
                    expected
                );
            }
            let population = state
                .frame
                .iter()
                .filter(|frame| !frame.particle.is_empty())
                .count();
            assert_eq!(index.retained(), width * 2 + count * 2 + population);
        }
    }
}

#[test]
fn population() {
    for width in [0, 1, 2, 64, 65, 257] {
        let original = (0..width)
            .map(|position| Token {
                id: position,
                value: if position % 3 == 0 {
                    Symbol::Atom(position % 7)
                } else {
                    Symbol::Rule(position % 7)
                },
                capture: (position % 3 != 0).then_some(position % 7),
            })
            .collect::<crate::population::Set>();
        let root = super::population::Index::default().advance(&original);
        let expected = root.reference(std::iter::empty());
        assert_eq!(root.retained(), 1);
        let mut partial = original.clone();
        partial.retain(|token| token.id % 2 == 1);
        let restored = root.advance(&partial).advance(&original);
        assert_eq!(restored.reference(std::iter::empty()), expected);
        assert_eq!(restored.retained(), 1 + expected.len());
        let mut value = original.clone();
        let mut index = root.advance(&value);
        for position in 0..width {
            let previous = value.clone();
            value.retain(|token| token.id != position);
            if position % 11 == 0 {
                value = value.iter().cloned().collect();
            }
            if position % 17 == 0 {
                value.reverse();
            }
            index = index.advance(&value);
            let target = value
                .iter()
                .filter_map(|token| token.capture)
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(index.reference(std::iter::empty()), target.clone().into());
            assert_eq!(root.reference(std::iter::empty()), expected);
            assert_eq!(
                index.retained(),
                1 + if previous.shared(&value) {
                    target.len()
                } else {
                    0
                }
            );
        }
        assert_eq!(index.reference(std::iter::empty()).len(), 0);
        let restored = index.advance(&original);
        assert_eq!(restored.reference(std::iter::empty()), expected);
        assert_eq!(restored.retained(), root.retained());
        let mut replacement = original.clone();
        for position in 0..replacement.len() {
            replacement[position].capture = Some(31);
        }
        let replaced = restored.advance(&replacement);
        assert_eq!(
            replaced
                .reference(std::iter::empty())
                .iter()
                .copied()
                .collect::<Vec<_>>(),
            if width == 0 { vec![] } else { vec![31] }
        );
        assert_eq!(restored.reference(std::iter::empty()), expected);
    }
}

#[test]
fn multiplicity() {
    let original = state(2);
    let mut state = original.clone();
    let root = super::dependency::Index::default().advance(&state, &[0, 1]);
    let mut index = root.clone();
    for iteration in 0..6 {
        let frame = Arc::make_mut(&mut state.frame[1]);
        match iteration {
            0 => frame.particle.retain(|token| token.id % 2 == 1),
            1 => frame.particle.clear(),
            2 => frame.held.clear(),
            3 => frame.parent = None,
            4 => frame.lexical = None,
            _ => state.frame[1] = original.frame[1].clone(),
        }
        index = index.advance(&state, &[1]);
        assert_eq!(index.dependent(0).any(|&frame| frame == 1), iteration != 4);
        assert!(root.dependent(0).any(|&frame| frame == 1));
        let fresh = super::dependency::Index::default().advance(&state, &[0, 1]);
        assert!(index.retained() >= fresh.retained());
        assert!(index.retained() <= fresh.retained() + 2);
        for target in 0..2 {
            assert_eq!(
                index.dependent(target).copied().collect::<Vec<_>>(),
                fresh.dependent(target).copied().collect::<Vec<_>>(),
            );
        }
    }
}

#[test]
fn eviction() {
    let mut state = state(65);
    let mut index = Index::new(Arc::new(state.clone()));
    let retained = index.retained();
    let released = index.evict();
    assert!(released > 0);
    assert_eq!(index.retained(), retained - released);
    assert_eq!(index.evict(), 0);
    assert_eq!(index.value, reference(&state));
    for iteration in 0..32 {
        let position = iteration * 7 % state.frame.len();
        let frame = Arc::make_mut(&mut state.frame[position]);
        frame.held[0].capture = Some(iteration);
        frame.particle.retain(|token| token.id % 3 != iteration % 3);
        let change = Change {
            world: Default::default(),
            insertion: state.world.len()..state.world.len(),
            frame: vec![position],
        };
        index = index.advance(
            Arc::new(state.clone()),
            &change,
            crate::layout::Layout::new(&state),
        );
        assert_eq!(index.value, reference(&state));
        assert_eq!(index.evict(), 0);
        let mut fresh = Index::new(Arc::new(state.clone()));
        fresh.evict();
        assert_eq!(index.retained(), fresh.retained());
    }
}
