use super::Index;
use crate::program::Symbol;
use crate::state::{Frame, State, Token, World};
use crate::term::Term;
use std::sync::Arc;

#[test]
fn intersection() {
    let mut seed = 71u64;
    let mut next = |bound| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as usize % bound
    };
    let mut state = State {
        world: (0..64)
            .map(|world| {
                World {
                    frame: world % 2,
                    particle: (0..4)
                        .map(|token| Token {
                            id: world * 4 + token,
                            value: Symbol::Atom(token),
                            capture: None,
                        })
                        .collect(),
                }
                .into()
            })
            .collect(),
        frame: (0..2)
            .map(|_| {
                Frame {
                    scope: 0,
                    parent: None,
                    lexical: None,
                    held: Vec::new(),
                }
                .into()
            })
            .collect(),
    };
    let mut index = Index::new(Arc::new(state.clone()));
    for iteration in 0..1024 {
        for width in 0..5 {
            let pattern = (0..width)
                .map(|_| {
                    if next(3) == 0 {
                        Term::new(Symbol::Rule(0), Some(next(2)))
                    } else {
                        Term::new(Symbol::Atom(next(4)), None)
                    }
                })
                .collect::<Vec<_>>();
            for frame in 0..2 {
                let expected = state
                    .world
                    .iter()
                    .enumerate()
                    .filter(|(_, world)| {
                        world.frame == frame
                            && pattern
                                .iter()
                                .all(|term| world.particle.iter().any(|token| term.matches(token)))
                    })
                    .map(|(world, _)| world)
                    .collect::<Vec<_>>();
                assert_eq!(index.candidate(&pattern, frame), expected);
            }
        }
        let removed = next(state.world.len());
        state.world.remove(removed);
        state.world.push(
            World {
                frame: next(2),
                particle: (0..next(5))
                    .map(|token| {
                        let value = if next(3) == 0 {
                            Symbol::Rule(0)
                        } else {
                            Symbol::Atom(next(4))
                        };
                        Token {
                            id: 256 + iteration * 4 + token,
                            value,
                            capture: matches!(value, Symbol::Rule(_)).then(|| next(2)),
                        }
                    })
                    .collect(),
            }
            .into(),
        );
        index.advance(Arc::new(state.clone()), &crate::basis::Set::single(removed));
    }
}

#[test]
fn batch() {
    let program = crate::program::Program::new(
        crate::lowering::parse("A.A.([A] B),A.B.([A] B),B.B,Empty").unwrap(),
    );
    let mut state = State::initial(&program);
    for _ in 0..8 {
        state.world.extend(state.world.clone().iter().cloned());
    }
    let mut index = Index::new(Arc::new(state.clone()));
    for iteration in 0..64 {
        let removed = (0..state.world.len())
            .filter(|world| (world + iteration) % 3 == 0)
            .collect::<crate::basis::Set<_>>();
        let mut replacement = removed
            .iter()
            .map(|&world| state.world[world].clone())
            .collect::<Vec<_>>();
        replacement.reverse();
        for &world in removed.iter().rev() {
            state.world.remove(world);
        }
        state.world.extend(replacement);
        index.advance(Arc::new(state.clone()), &removed);
        for symbol in program
            .atom
            .iter()
            .enumerate()
            .map(|(index, _)| Symbol::Atom(index))
            .chain(program.code.keys().copied().map(Symbol::Rule))
        {
            for capture in [None, Some(0), Some(1)] {
                let pattern = [Term::new(symbol, capture)];
                let expected = state
                    .world
                    .iter()
                    .enumerate()
                    .filter_map(|(world, value)| {
                        value
                            .particle
                            .iter()
                            .any(|token| pattern[0].matches(token))
                            .then_some(world)
                    })
                    .collect::<Vec<_>>();
                assert_eq!(index.candidate(&pattern, 0), expected);
                for (world, value) in state.world.iter().enumerate() {
                    assert_eq!(
                        index.quantity(&pattern[0], 0, index.site(world)),
                        value
                            .particle
                            .iter()
                            .filter(|token| pattern[0].matches(token))
                            .count()
                    );
                }
            }
        }
        assert_eq!(
            index.retained,
            index
                .frame
                .iter()
                .map(|frame| frame.iter().count())
                .sum::<usize>()
                + index.reader.iter().map(Vec::len).sum::<usize>()
                + index.term.values().map(Vec::len).sum::<usize>()
        );
        let fresh = Index::new(Arc::new(state.clone()));
        let read = |index: &Index| {
            let mut value = index
                .reader(0)
                .iter()
                .map(|reader| {
                    (
                        index.world(reader.read.site),
                        reader.read.resource,
                        reader.rule,
                        reader.owner,
                    )
                })
                .collect::<Vec<_>>();
            value.sort_unstable();
            value
        };
        assert_eq!(read(&index), read(&fresh));
    }
}
