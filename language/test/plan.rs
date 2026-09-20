use super::Input;
use crate::program::Symbol;
use crate::state::Token;
use std::sync::Arc;
use std::task::Poll;

#[test]
fn sharing() {
    let mut shared = Default::default();
    let left = Input::shared(&[vec![Symbol::Rule(0)], vec![Symbol::Atom(0)]], &mut shared);
    let right = Input::shared(&[vec![Symbol::Rule(0)], vec![Symbol::Atom(1)]], &mut shared);
    assert!(Arc::ptr_eq(&left.fragment[0], &right.fragment[0]));
    assert!(!Arc::ptr_eq(&left.fragment[1], &right.fragment[1]));
    let particle = [Token {
        id: 7,
        value: Symbol::Rule(0),
        capture: Some(1),
    }];
    let mut accepted = left.context(1).prepare(0, &particle);
    let mut rejected = right.context(2).prepare(0, &particle);
    assert_eq!(accepted.step(), Poll::Ready(Some(vec![7])));
    assert_eq!(rejected.step(), Poll::Ready(None));
    let mut later = right.context(1).prepare(0, &particle);
    assert_eq!(later.step(), Poll::Ready(Some(vec![7])));
}

#[test]
fn predicate() {
    let program = crate::program::Program::new(
        crate::lowering::parse("A.A.B,A.B.B,A.A.([A] B),B.([A] B)").unwrap(),
    );
    let state = Arc::new(crate::state::State::initial(&program));
    let index = crate::index::Index::new(state.clone());
    for encoding in 0..256usize {
        let value = (0..4)
            .map(|position| match (encoding >> (position * 2)) & 3 {
                0 => Symbol::Atom(0),
                1 => Symbol::Atom(1),
                _ => Symbol::Rule(*program.code.keys().next().unwrap()),
            })
            .collect::<Vec<_>>();
        let input = Input::new(&[value]);
        for owner in 0..2 {
            let context = input.context(owner);
            let pattern = input.pattern(owner);
            let expected = state
                .world
                .iter()
                .enumerate()
                .filter_map(|(position, world)| {
                    let expected = pattern[0]
                        .iter()
                        .all(|term| world.particle.iter().any(|token| term.matches(token)));
                    assert_eq!(context.matches(0, &index, index.site(position)), expected);
                    expected.then_some(position)
                })
                .collect::<Vec<_>>();
            assert_eq!(context.candidate(0, &index, 0), expected);
        }
    }
}

#[test]
fn mutation() {
    let program = crate::program::Program::new(
        crate::lowering::parse("A.A.B,A.B.B,A.A.([A] B),B.([A] B)").unwrap(),
    );
    let mut state = crate::state::State::initial(&program);
    state.frame.push(state.frame[0].clone());
    for position in 0..state.world.len() {
        let world = Arc::make_mut(&mut state.world[position]);
        world.frame = position % 2;
        world.particle.extend((0..64).map(|offset| Token {
            id: 1000 + position * 64 + offset,
            value: if offset % 3 == 0 {
                Symbol::Rule(*program.code.keys().next().unwrap())
            } else {
                Symbol::Atom(offset % 2)
            },
            capture: Some(offset % 2),
        }));
    }
    let mut index = crate::index::Index::new(Arc::new(state.clone()));
    for iteration in 0..64 {
        for owner in 0..2 {
            for width in [0, 1, 4, 8, 32] {
                let value = (0..width)
                    .map(|position| match (position + iteration) % 4 {
                        0 => Symbol::Atom(0),
                        1 => Symbol::Atom(1),
                        2 => Symbol::Rule(*program.code.keys().next().unwrap()),
                        _ => Symbol::Atom(100),
                    })
                    .collect::<Vec<_>>();
                let input = Input::new(&[value]);
                let context = input.context(owner);
                let pattern = input.pattern(owner);
                for (world, value) in state.world.iter().enumerate() {
                    let expected = pattern[0]
                        .iter()
                        .all(|term| value.particle.iter().any(|token| term.matches(token)));
                    assert_eq!(context.matches(0, &index, index.site(world)), expected);
                }
            }
        }
        let removed = iteration % state.world.len();
        let mut world = (*state.world.remove(removed)).clone();
        world.frame = 1 - world.frame;
        for token in &mut world.particle {
            token.capture = token.capture.map(|capture| 1 - capture);
        }
        if iteration % 2 == 0 {
            world.particle.truncate(8);
        }
        state.world.push(world.into());
        index.advance(Arc::new(state.clone()), &crate::basis::Set::single(removed));
    }
}

#[test]
fn summary() {
    let program = crate::program::Program::new(
        crate::lowering::parse("A.A.A.A.A.A.B.B.([A] B).([A] B),A.A,B.B,B.([A] B)").unwrap(),
    );
    let state = Arc::new(crate::state::State::initial(&program));
    let index = crate::index::Index::new(state.clone());
    for width in 0..10 {
        for count in 0..=width {
            for symbol in [
                Symbol::Atom(1),
                Symbol::Rule(*program.code.keys().next().unwrap()),
            ] {
                let value = std::iter::repeat_n(Symbol::Atom(0), count)
                    .chain(std::iter::repeat_n(symbol, width - count))
                    .collect::<Vec<_>>();
                let input = Input::new(&[value]);
                for owner in 0..2 {
                    let context = input.context(owner);
                    let pattern = input.pattern(owner);
                    for (world, value) in state.world.iter().enumerate() {
                        let mut actual = context.select(0, &index, index.site(world));
                        let mut expected =
                            crate::particle::Match::new(&pattern[0], &value.particle);
                        for _ in 0..2 {
                            loop {
                                let next = actual.step();
                                assert_eq!(next, expected.step());
                                if matches!(next, Poll::Ready(None)) {
                                    break;
                                }
                            }
                            actual.reset();
                            expected.reset();
                        }
                    }
                }
            }
        }
    }
}
