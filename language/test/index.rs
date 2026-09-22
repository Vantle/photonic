use super::Index;
use crate::program::Symbol;
use crate::state::{Frame, State, Token, World};
use crate::term::Term;
use std::sync::Arc;

fn verify(index: &Index) {
    use crate::location::Location;
    let state = &index.state;
    let fresh = Index::new(state.clone());
    assert_eq!(index.reach.frame, fresh.reach.frame);
    assert_eq!(index.symbol, fresh.symbol);
    assert_eq!(index.lexicon, fresh.lexicon);
    assert_eq!(index.retained, fresh.retained);
    for frame in index.frame() {
        let mut visible = Vec::new();
        let mut cursor = Some(frame);
        while let Some(owner) = cursor {
            visible.extend(&state.frame[owner].particle);
            cursor = state.frame[owner].lexical;
        }
        for pattern in std::iter::once(Vec::new()).chain((0..4).flat_map(|value| {
            (0..state.frame.len()).flat_map(move |capture| {
                let term = Term::new(Symbol::Rule(value), Some(capture));
                [
                    vec![term.clone()],
                    vec![Term::new(Symbol::Atom(0), None), term.clone()],
                    vec![term.clone(), term],
                ]
            })
        })) {
            let expected = state
                .world
                .iter()
                .enumerate()
                .filter_map(|(world, value)| {
                    (value.frame == frame
                        && pattern.iter().all(|term| {
                            value
                                .particle
                                .iter()
                                .chain(visible.iter().copied())
                                .any(|token| term.matches(token))
                        }))
                    .then_some(Location::World(world))
                })
                .chain(
                    (!pattern.is_empty()
                        && pattern
                            .iter()
                            .all(|term| visible.iter().any(|token| term.matches(token))))
                    .then_some(Location::Context(frame)),
                )
                .collect::<Vec<_>>();
            let actual = index
                .candidate(pattern.clone(), frame)
                .into_iter()
                .map(|site| index.location(site))
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "frame {frame}, pattern {pattern:?}");
            for location in actual {
                let site = index.locate(location);
                for term in &pattern {
                    let count = location
                        .world()
                        .into_iter()
                        .flat_map(|world| &state.world[world].particle)
                        .chain(visible.iter().copied())
                        .filter(|token| term.matches(token))
                        .count();
                    assert_eq!(index.quantity(term, frame, site), count);
                }
            }
        }
    }
}

#[test]
fn population() {
    for width in [1, 63, 64, 65, 129, 1025] {
        let program = crate::program::Program::new(crate::lowering::parse("A [A] B").unwrap());
        let mut state = State::initial(&program);
        Arc::make_mut(&mut state.frame[0]).particle = (0..width)
            .map(|position| Token {
                id: position + 1,
                value: Symbol::Rule(position % 4),
                capture: Some(0),
            })
            .collect();
        let original = state.frame[0].clone();
        let mut index = Index::new(Arc::new(state.clone()));
        let site = index.locate(crate::location::Location::Context(0));
        let change = crate::change::Change {
            world: Default::default(),
            insertion: state.world.len()..state.world.len(),
            frame: vec![0],
        };
        for divisor in [2, 3, 7] {
            Arc::make_mut(&mut state.frame[0])
                .particle
                .retain(|token| token.id % divisor != 0);
            index.update(Arc::new(state.clone()), &change);
            verify(&index);
            assert_eq!(index.locate(crate::location::Location::Context(0)), site);
        }
        let particle = state.frame[0].particle.iter().cloned().collect();
        Arc::make_mut(&mut state.frame[0]).particle = particle;
        index.update(Arc::new(state.clone()), &change);
        verify(&index);
        assert!(index.context.is_empty());
        assert_eq!(index.locate(crate::location::Location::Context(0)), site);
        state.frame[0] = original.clone();
        index.update(Arc::new(state.clone()), &change);
        verify(&index);
        assert_eq!(index.locate(crate::location::Location::Context(0)), site);
        Arc::make_mut(&mut state.frame[0]).particle.clear();
        index.update(Arc::new(state.clone()), &change);
        verify(&index);
        assert_eq!(index.locate(crate::location::Location::Context(0)), site);
        state.frame[0] = original;
        index.update(Arc::new(state.clone()), &change);
        verify(&index);
        for symbol in (0..4).map(Symbol::Rule) {
            let expected = state.frame[0]
                .particle
                .iter()
                .filter(|token| token.value == symbol)
                .map(|token| token.id)
                .collect::<Vec<_>>();
            assert_eq!(
                index
                    .occurrence(0, symbol)
                    .map(|(_, token)| token.id)
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}

#[test]
fn context() {
    let program = crate::program::Program::new(crate::lowering::parse("A [A] B").unwrap());
    let mut state = State::initial(&program);
    let root = state.frame[0].clone();
    state.frame = (0..8)
        .map(|frame| {
            let mut value = (*root).clone();
            value.parent = (frame > 0).then_some(0);
            value.lexical = (frame > 0).then_some(frame / 2);
            value.particle = vec![Token {
                id: frame + 100,
                value: Symbol::Rule(frame % 4),
                capture: Some(frame),
            }]
            .into();
            value.into()
        })
        .collect();
    let world = state.world[0].clone();
    state.world = (0..64)
        .map(|position| {
            let mut value = (*world).clone();
            value.frame = position % 8;
            value.particle[0].id = position;
            value.into()
        })
        .collect();
    let mut index = Index::new(Arc::new(state.clone()));
    for iteration in 0..128 {
        verify(&index);
        let original = index.coherence.clone();
        let frame = iteration % 8;
        let value = Arc::make_mut(&mut state.frame[frame]);
        value.lexical = (frame > 0).then_some(iteration / 8 % frame.max(1));
        value.particle = if iteration % 3 == 0 {
            Default::default()
        } else {
            vec![Token {
                id: 200 + iteration,
                value: Symbol::Rule(iteration % 4),
                capture: Some(iteration / 4 % 8),
            }]
            .into()
        };
        index.update(
            Arc::new(state.clone()),
            &crate::change::Change {
                world: Default::default(),
                insertion: state.world.len()..state.world.len(),
                frame: vec![frame],
            },
        );
        assert_eq!(index.coherence, original);
        for (world, &site) in original.iter().enumerate() {
            assert_eq!(
                index.removal.contains(&site),
                index.context.contains(&state.world[world].frame)
            );
        }
    }
    verify(&index);
}

#[test]
fn retirement() {
    let program = crate::program::Program::new(crate::lowering::parse("A [A] B").unwrap());
    let initial = State::initial(&program);
    let mut index = Index::new(Arc::new(initial.clone()));
    for width in [4, 2, 8, 1, 3, 1] {
        let mut state = initial.clone();
        state.frame = (0..width)
            .map(|frame| {
                let mut value = (*initial.frame[0]).clone();
                value.parent = (frame > 0).then_some(0);
                value.lexical = (frame > 0).then_some(0);
                value.particle[0].id += frame * 2;
                value.particle[0].capture = Some(frame);
                value.into()
            })
            .collect();
        state.world = (0..width)
            .map(|frame| {
                let mut world = (*initial.world[0]).clone();
                world.frame = frame;
                world.particle[0].id += frame * 2;
                world.into()
            })
            .collect();
        let change = crate::change::Change {
            world: (0..index.state.world.len()).collect(),
            insertion: 0..width,
            frame: (0..index.state.frame.len().max(width)).collect(),
        };
        index.update(Arc::new(state), &change);
        verify(&index);
        assert_eq!(index.location.iter().flatten().count(), width * 2);
    }
}

#[test]
fn skew() {
    let state = State {
        world: (0..4096)
            .map(|world| {
                World {
                    frame: 0,
                    particle: [1, 2, 64, 127]
                        .into_iter()
                        .enumerate()
                        .filter(|&(_, divisor)| world % divisor == 0)
                        .map(|(position, _)| Token {
                            id: world * 4 + position,
                            value: Symbol::Atom(position),
                            capture: None,
                        })
                        .collect(),
                }
                .into()
            })
            .collect(),
        frame: vec![
            Frame {
                scope: 0,
                parent: None,
                lexical: None,
                particle: Default::default(),
                held: Vec::new(),
            }
            .into(),
        ]
        .into(),
    };
    let index = Index::new(Arc::new(state.clone()));
    for order in crate::ordering::Ordering::new(0..4, |_| 0) {
        for width in 1..=4 {
            let pattern = order[..width]
                .iter()
                .map(|&value| Term::new(Symbol::Atom(value), None))
                .collect::<Vec<_>>();
            let expected = state
                .world
                .iter()
                .enumerate()
                .filter(|(_, world)| {
                    pattern
                        .iter()
                        .all(|term| world.particle.iter().any(|token| term.matches(token)))
                })
                .map(|(world, _)| world)
                .collect::<Vec<_>>();
            assert_eq!(
                index
                    .candidate(pattern.iter().cloned(), 0)
                    .into_iter()
                    .map(|site| index.world(site))
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}

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
                    particle: Default::default(),
                    held: Vec::new(),
                }
                .into()
            })
            .collect(),
    };
    let mut index = Index::new(Arc::new(state.clone()));
    for iteration in 0..1024 {
        for world in 0..state.world.len() {
            let site = index.site(world);
            assert!(!index.precedes(site, site));
            if world > 0 {
                let previous = index.site(world - 1);
                assert!(index.precedes(previous, site));
                assert!(!index.precedes(site, previous));
            }
        }
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
                assert_eq!(
                    index
                        .candidate(pattern.iter().cloned(), frame)
                        .into_iter()
                        .map(|site| index.world(site))
                        .collect::<Vec<_>>(),
                    expected
                );
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
            .chain((0..program.rule.len()).map(Symbol::Rule))
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
                assert_eq!(
                    index
                        .candidate(pattern.iter().cloned(), 0)
                        .into_iter()
                        .map(|site| index.world(site))
                        .collect::<Vec<_>>(),
                    expected
                );
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
                .map(|reader| (reader.read.place(index), reader.rule, reader.owner))
                .collect::<Vec<_>>();
            value.sort_unstable();
            value
        };
        assert_eq!(read(&index), read(&fresh));
    }
}
