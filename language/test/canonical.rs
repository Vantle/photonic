use super::Search;
use crate::ordering::Ordering;
use crate::program::Symbol;
use crate::refinement::Refinement;
use crate::state::{Canonical, Frame, State, Token, World};
use std::sync::Arc;

fn reference(state: &State) -> (Canonical, usize) {
    let refinement = Refinement::new(state);
    let world = Ordering::new(0..state.world.len(), |index| {
        let world = &state.world[index];
        let mut particle = world
            .particle
            .iter()
            .map(|token| token.value)
            .collect::<Vec<_>>();
        particle.sort();
        (state.chain(world.frame), particle, refinement.world[index])
    });
    let world = if state.world.len() >= 4 {
        world.quotient(|| crate::symmetry::world(state))
    } else {
        world
    };
    let mut best: Option<Canonical> = None;
    let mut work = 1;
    for world in world {
        work += 1;
        let mut occupied = vec![Vec::new(); state.frame.len()];
        let mut capture = vec![Vec::new(); state.frame.len()];
        for (position, &source) in world.iter().enumerate() {
            let value = &state.world[source];
            occupied[value.frame].push(position);
            for token in &value.particle {
                if let Some(frame) = token.capture {
                    capture[frame].push((position, token.value));
                }
            }
        }
        for capture in &mut capture {
            capture.sort_unstable();
        }
        for frame in Ordering::new(
            state.reachable().into_iter().filter(|&index| index != 0),
            |index| {
                (
                    state.chain(index),
                    &occupied[index],
                    &capture[index],
                    refinement.frame[index],
                )
            },
        ) {
            work += 1;
            let order = std::iter::once(0).chain(frame).collect::<Vec<_>>();
            let value = state.rename(&world, &order);
            if best.as_ref().is_none_or(|best| value.state < best.state) {
                best = Some(value);
            }
        }
    }
    (best.unwrap(), work)
}

fn verify(state: State) {
    let incidence = crate::incidence::Incidence::new(&state);
    let frame = state.reachable();
    for world in [
        (0..state.world.len()).collect::<Vec<_>>(),
        (0..state.world.len()).rev().collect(),
    ] {
        for frame in [
            frame.clone(),
            std::iter::once(0)
                .chain(frame[1..].iter().rev().copied())
                .collect(),
        ] {
            let expected = state.rename(&world, &frame);
            let actual = super::renaming::rename(&state, &incidence, &world, &frame);
            assert_eq!(actual.state, expected.state);
            assert_eq!(actual.world, expected.world);
            assert_eq!(actual.frame, expected.frame);
            assert_eq!(actual.resource, expected.resource);
        }
    }
    let (expected, work) = reference(&state);
    let mut search = Search::new(Arc::new(state));
    for position in 1..=work {
        assert_eq!(search.step(), position == work);
    }
    assert!(search.step());
    let actual = search.finish().unwrap();
    assert_eq!(actual.state, expected.state);
    assert_eq!(actual.world, expected.world);
    assert_eq!(actual.frame, expected.frame);
    assert_eq!(actual.resource, expected.resource);
}

#[test]
fn membership() {
    let particle = vec![
        Token {
            id: usize::MAX,
            value: Symbol::Rule(0),
            capture: Some(2),
        },
        Token {
            id: 7,
            value: Symbol::Atom(0),
            capture: None,
        },
        Token {
            id: 101,
            value: Symbol::Atom(0),
            capture: None,
        },
    ];
    let state = State {
        world: vec![
            Arc::new(World {
                frame: 1,
                particle: particle.iter().rev().cloned().collect(),
            }),
            Arc::new(World {
                frame: 2,
                particle: particle.iter().chain(&particle).cloned().collect(),
            }),
        ]
        .into(),
        frame: (0..4)
            .map(|index| {
                Arc::new(Frame {
                    scope: index,
                    parent: (index > 0).then_some(0),
                    lexical: (index > 0).then_some(index.saturating_sub(1)),
                    particle: particle.iter().cloned().collect(),
                    held: particle.clone(),
                })
            })
            .collect(),
    };
    assert_eq!(state.reachable(), vec![0, 1, 2]);
    verify(state);
}

#[test]
fn demand() {
    let program = crate::program::Program::new(&crate::lowering::parse("A,B [A] C").unwrap());
    let state = State::initial(&program);
    let mut search = Search::new(Arc::new(state.clone()));
    assert!(search.refinement.get().is_none());
    while !search.step() {
        assert!(search.refinement.get().is_none());
    }
    assert!(search.refinement.get().is_none());
    verify(state);
    let program = crate::program::Program::new(&crate::lowering::parse("A,A").unwrap());
    let state = State::initial(&program);
    let search = Search::new(Arc::new(state.clone()));
    assert!(search.refinement.get().is_some());
    verify(state);
    let state = State {
        world: Default::default(),
        frame: (0..3)
            .map(|index| {
                Arc::new(Frame {
                    scope: 0,
                    parent: (index > 0).then_some(0),
                    lexical: (index > 0).then_some(0),
                    held: Vec::new(),
                    particle: if index == 0 {
                        (1..3)
                            .map(|capture| Token {
                                id: capture,
                                value: Symbol::Rule(capture),
                                capture: Some(capture),
                            })
                            .collect()
                    } else {
                        Default::default()
                    },
                })
            })
            .collect(),
    };
    let mut search = Search::new(Arc::new(state.clone()));
    assert!(search.refinement.get().is_none());
    assert!(!search.step());
    assert!(search.refinement.get().is_some());
    verify(state);
}

#[test]
fn differential() {
    let mut seed = 7u64;
    for iteration in 0..256 {
        let count = 1 + iteration % 3;
        let mut select = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            seed >> 61 == 0
        };
        let token = (0..8)
            .map(|index| Token {
                id: 17 + index * 11,
                value: if index % 3 == 0 {
                    Symbol::Rule(index % 2)
                } else {
                    Symbol::Atom(index % 3)
                },
                capture: (index % 3 == 0).then_some((index + iteration) % count),
            })
            .collect::<Vec<_>>();
        let frame = (0..count)
            .map(|index| {
                Arc::new(Frame {
                    scope: (index + iteration) % 2,
                    parent: (index > 0).then_some(0),
                    lexical: (index > 0).then_some(0),
                    particle: token.iter().filter(|_| select()).cloned().collect(),
                    held: token.iter().filter(|_| select()).cloned().collect(),
                })
            })
            .collect();
        let world = (0..iteration % 5)
            .map(|index| {
                Arc::new(World {
                    frame: index % count,
                    particle: token.iter().filter(|_| select()).cloned().collect(),
                })
            })
            .collect();
        let state = State { world, frame };
        verify(state.clone());
        verify(State {
            world: state.world.iter().rev().cloned().collect(),
            ..state
        });
    }
}
