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
