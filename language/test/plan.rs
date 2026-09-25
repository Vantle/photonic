use super::Input;
use crate::program::Symbol;
use crate::state::Token;
use std::sync::Arc;

#[test]
fn ownership() {
    let input = Input::new(&[
        vec![Symbol::Atom(2), Symbol::Rule(7), Symbol::Atom(2)],
        vec![Symbol::Rule(7)],
        vec![Symbol::Atom(2), Symbol::Rule(7), Symbol::Atom(2)],
    ]);
    let shape = Arc::downgrade(&input.shape);
    let fragment = Arc::downgrade(&input.shape.fragment[0]);
    let left = input.context(1);
    let right = input.context(usize::MAX);
    assert_eq!(shape.strong_count(), 3);
    drop(input);
    assert_eq!(shape.strong_count(), 2);
    assert_eq!(left.group(), &[0, 1, 0]);
    assert_eq!(right.group(), left.group());
    assert_eq!(
        right.pattern(0).collect::<Vec<_>>(),
        vec![
            crate::term::Term::new(Symbol::Atom(2), None),
            crate::term::Term::new(Symbol::Rule(7), Some(usize::MAX)),
            crate::term::Term::new(Symbol::Atom(2), None),
        ]
    );
    let particle = [Token {
        id: 19,
        value: Symbol::Rule(7),
        capture: Some(1),
    }];
    assert_eq!(left.prepare(1, &particle).step(), Some(vec![19]));
    assert_eq!(right.prepare(1, &particle).step(), None);
    drop(left);
    assert_eq!(shape.strong_count(), 1);
    assert!(fragment.upgrade().is_some());
    drop(right);
    assert!(shape.upgrade().is_none());
    assert!(fragment.upgrade().is_none());
}

#[test]
fn sharing() {
    let mut shared = Default::default();
    let left = Input::shared(
        &[vec![Symbol::Rule(0)], vec![Symbol::Atom(0)]],
        |_| true,
        &mut shared,
    );
    let right = Input::shared(
        &[vec![Symbol::Rule(0)], vec![Symbol::Atom(1)]],
        |_| true,
        &mut shared,
    );
    assert!(Arc::ptr_eq(
        &left.shape.fragment[0],
        &right.shape.fragment[0]
    ));
    assert!(!Arc::ptr_eq(
        &left.shape.fragment[1],
        &right.shape.fragment[1]
    ));
    let particle = [Token {
        id: 7,
        value: Symbol::Rule(0),
        capture: Some(1),
    }];
    let mut accepted = left.context(1).prepare(0, &particle);
    let mut rejected = right.context(2).prepare(0, &particle);
    assert_eq!(accepted.step(), Some(vec![7]));
    assert_eq!(rejected.step(), None);
    let mut later = right.context(1).prepare(0, &particle);
    assert_eq!(later.step(), Some(vec![7]));
}

#[test]
fn predicate() {
    let program = crate::program::Program::new(
        &crate::lowering::parse("A.A.B,A.B.B,A.A.([A] B),B.([A] B)").unwrap(),
    );
    let state = Arc::new(crate::state::State::initial(&program));
    let index = crate::index::Index::new(state.clone());
    for encoding in 0..256usize {
        let value = (0..4)
            .map(|position| match (encoding >> (position * 2)) & 3 {
                0 => Symbol::Atom(0),
                1 => Symbol::Atom(1),
                _ => Symbol::Rule(0),
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
            assert_eq!(
                context
                    .candidate(0, &index, 0)
                    .into_iter()
                    .map(|site| index.world(site))
                    .collect::<Vec<_>>(),
                expected
            );
        }
    }
}

#[test]
fn mutation() {
    let program = crate::program::Program::new(
        &crate::lowering::parse("A.A.B,A.B.B,A.A.([A] B),B.([A] B)").unwrap(),
    );
    let mut state = crate::state::State::initial(&program);
    state.frame.push(state.frame[0].clone());
    for position in 0..state.world.len() {
        let world = Arc::make_mut(&mut state.world[position]);
        world.frame = position % 2;
        world.particle.extend((0..64).map(|offset| Token {
            id: 1000 + position * 64 + offset,
            value: if offset % 3 == 0 {
                Symbol::Rule(0)
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
                        2 => Symbol::Rule(0),
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
        &crate::lowering::parse("A.A.A.A.A.A.B.B.([A] B).([A] B),A.A,B.B,B.([A] B)").unwrap(),
    );
    let state = Arc::new(crate::state::State::initial(&program));
    let index = crate::index::Index::new(state.clone());
    for width in 0..10 {
        for count in 0..=width {
            for symbol in [Symbol::Atom(1), Symbol::Rule(0)] {
                let value = std::iter::repeat_n(Symbol::Atom(0), count)
                    .chain(std::iter::repeat_n(symbol, width - count))
                    .collect::<Vec<_>>();
                let input = Input::new(&[value]);
                for owner in 0..2 {
                    let context = input.context(owner);
                    let pattern = input.pattern(owner);
                    for (world, value) in state.world.iter().enumerate() {
                        let mut actual = context.select(0, &index, index.site(world), None);
                        let mut expected =
                            crate::particle::Match::new(&pattern[0], &value.particle);
                        for _ in 0..2 {
                            loop {
                                let next = actual.step();
                                assert_eq!(next, expected.step());
                                if next.is_none() {
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

#[test]
fn rejection() {
    let program = crate::program::Program::new(&crate::lowering::parse("A.([A] B)").unwrap());
    let mut seed = 71u64;
    let mut next = |bound| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as usize % bound
    };
    let symbol = |value| match value {
        0 => Symbol::Rule(0),
        value => Symbol::Atom(value),
    };
    for iteration in 0..512 {
        let mut state = crate::state::State::initial(&program);
        state.frame.push(state.frame[0].clone());
        let world = Arc::make_mut(&mut state.world[0]);
        world.particle = (0..next(48))
            .map(|id| {
                let value = symbol(next(8));
                Token {
                    id,
                    value,
                    capture: matches!(value, Symbol::Rule(_)).then(|| next(2)),
                }
            })
            .collect();
        if iteration % 2 == 0 {
            world.particle.extend(world.particle.clone());
        }
        let pattern = (0..next(10)).map(|_| symbol(next(8))).collect::<Vec<_>>();
        let input = Input::new(&[pattern]);
        let index = crate::index::Index::new(Arc::new(state.clone()));
        for owner in 0..2 {
            let context = input.context(owner);
            let mut actual = context.select(0, &index, index.site(0), None);
            let mut expected =
                crate::particle::Match::new(&input.pattern(owner)[0], &state.world[0].particle);
            assert_eq!(actual.viable(), expected.viable());
            for _ in 0..2 {
                for _ in 0..512 {
                    let result = expected.step();
                    assert_eq!(actual.step(), result);
                    if result.is_none() {
                        break;
                    }
                }
                actual.reset();
                expected.reset();
            }
        }
    }
}
