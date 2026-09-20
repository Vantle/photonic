use super::{Request, Store};
use crate::factor::Budget;
use crate::particle::Match;
use crate::pattern::Pattern;
use crate::program::Symbol;
use crate::state::{Token, World};
use crate::term::Term;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::Poll;

#[test]
fn isolation() {
    let accounting = Arc::new(AtomicUsize::new(0));
    let store = Store::new(Arc::new(Budget::new(4096)), accounting.clone());
    let mut seed = 43u64;
    let mut next = |bound| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as usize % bound
    };
    for iteration in 0..256 {
        let pattern = (0..next(5))
            .map(|_| match next(3) {
                0 => Symbol::Rule(0),
                value => Symbol::Atom(value),
            })
            .collect::<Vec<_>>();
        let fragment = Arc::new(Pattern::new(&pattern));
        let world = Arc::new(World {
            frame: 0,
            particle: (0..next(8))
                .map(|id| {
                    let value = match next(3) {
                        0 => Symbol::Rule(0),
                        value => Symbol::Atom(value),
                    };
                    Token {
                        id: iteration * 8 + id,
                        value,
                        capture: matches!(value, Symbol::Rule(_)).then(|| next(2)),
                    }
                })
                .collect(),
        });
        for owner in 0..2 {
            let request = || Request {
                pattern: &fragment,
                world: &world,
                owner,
            };
            let mut left = store.select(request());
            let mut right = store.select(request());
            assert!(Arc::ptr_eq(&left.preparation(), &right.preparation()));
            let pattern = pattern
                .iter()
                .map(|&value| Term::new(value, Some(owner)))
                .collect::<Vec<_>>();
            let mut reference = Match::new(&pattern, &world.particle);
            for pass in 0..3 {
                loop {
                    let expected = reference.step();
                    assert_eq!(left.step(), expected);
                    if pass == 1 {
                        store.evict();
                        assert_eq!(accounting.load(Ordering::Relaxed), 0);
                    }
                    assert_eq!(right.step(), expected);
                    if matches!(expected, Poll::Ready(None)) {
                        break;
                    }
                }
                reference.reset();
                left.reset();
                right.reset();
            }
        }
    }
    store.evict();
    assert_eq!(accounting.load(Ordering::Relaxed), 0);
}

#[test]
fn identity() {
    let accounting = Arc::new(AtomicUsize::new(0));
    let store = Store::new(Arc::new(Budget::new(4096)), accounting.clone());
    let pattern = Arc::new(Pattern::new(&[Symbol::Rule(0)]));
    let mut world = Arc::new(World {
        frame: 0,
        particle: vec![Token {
            id: 0,
            value: Symbol::Rule(0),
            capture: Some(0),
        }],
    });
    let mut original = store.select(Request {
        pattern: &pattern,
        world: &world,
        owner: 0,
    });
    let mut foreign = store.select(Request {
        pattern: &pattern,
        world: &world,
        owner: 1,
    });
    assert_eq!(original.step(), Poll::Ready(Some(vec![0])));
    assert_eq!(foreign.step(), Poll::Ready(None));
    Arc::make_mut(&mut world).particle[0].id = 100;
    let mut replacement = store.select(Request {
        pattern: &pattern,
        world: &world,
        owner: 0,
    });
    assert_eq!(replacement.step(), Poll::Ready(Some(vec![100])));
    assert!(!Arc::ptr_eq(
        &original.preparation(),
        &replacement.preparation()
    ));
    original.reset();
    assert_eq!(original.step(), Poll::Ready(Some(vec![0])));
    store.evict();
    assert_eq!(accounting.load(Ordering::Relaxed), 0);
}

#[test]
fn saturation() {
    for capacity in [0, 1, 4, 16, 64] {
        let accounting = Arc::new(AtomicUsize::new(0));
        let store = Store::new(Arc::new(Budget::new(capacity)), accounting.clone());
        let pattern = Arc::new(Pattern::new(&[Symbol::Atom(0); 2]));
        for id in 0..128 {
            let world = Arc::new(World {
                frame: 0,
                particle: (0..4)
                    .map(|offset| Token {
                        id: id * 4 + offset,
                        value: Symbol::Atom(0),
                        capture: None,
                    })
                    .collect(),
            });
            let mut search = store.select(Request {
                pattern: &pattern,
                world: &world,
                owner: id,
            });
            let mut reference = Match::prepared(&pattern, Some(id), &world.particle);
            loop {
                let expected = reference.step();
                assert_eq!(search.step(), expected);
                if matches!(expected, Poll::Ready(None)) {
                    break;
                }
            }
            assert!(accounting.load(Ordering::Relaxed) <= capacity);
        }
        store.evict();
        assert_eq!(accounting.load(Ordering::Relaxed), 0);
    }
}

#[test]
fn subscription() {
    use crate::index::Index;
    use crate::joining::Join;
    use crate::plan::Input;
    use crate::program::Program;
    use crate::state::State;

    let program = Program::new(
        crate::lowering::parse(
            "A.B.C.D.E.F.G.H,X,Y [A.B.C.D.E.F.G.H,X] End [A.B.C.D.E.F.G.H,Y] End",
        )
        .unwrap(),
    );
    let mut state = State::initial(&program);
    let position = state
        .world
        .iter()
        .position(|world| world.particle.len() == 8)
        .unwrap();
    Arc::make_mut(&mut state.world[position])
        .particle
        .extend((0..64).map(|offset| Token {
            id: 100 + offset,
            value: Symbol::Atom(usize::MAX),
            capture: None,
        }));
    let mut index = Index::new(Arc::new(state.clone()));
    let store = Arc::new(crate::joining::Store::new(65536));
    let mut fragment = Default::default();
    let input = program
        .rule
        .iter()
        .map(|rule| Input::shared(&rule.input, &mut fragment))
        .collect::<Vec<_>>();
    let mut search = input
        .iter()
        .map(|input| {
            (
                Join::planned(crate::joining::Request {
                    input,
                    index: &index,
                    frame: 0,
                    owner: 0,
                    store: &store,
                }),
                Join::new(input.pattern(0), &index, 0),
            )
        })
        .collect::<Vec<_>>();
    for iteration in 0..32 {
        let mut complete = vec![false; search.len()];
        for step in 0..100 {
            for ((actual, expected), complete) in search.iter_mut().zip(&mut complete) {
                if *complete {
                    continue;
                }
                let result = actual.step(&index);
                assert_eq!(result, expected.step(&index));
                *complete = matches!(result, Poll::Ready(None));
            }
            if iteration % 3 == 0 && step == 2 {
                store.evict();
                for (actual, _) in &mut search {
                    actual.evict();
                }
            }
            if complete.iter().all(|&complete| complete) {
                break;
            }
        }
        assert!(complete.iter().all(|&complete| complete));
        let position = state
            .world
            .iter()
            .position(|world| world.particle.len() > 8)
            .unwrap();
        let mut world = (*state.world.remove(position)).clone();
        for token in &mut world.particle {
            token.id += 1000;
        }
        state.world.push(world.into());
        index.advance(
            Arc::new(state.clone()),
            &crate::basis::Set::single(position),
        );
        for (actual, expected) in &mut search {
            actual.advance(&index);
            expected.advance(&index);
        }
    }
    drop(search);
    store.evict();
    assert!(store.budget().reserve(65536));
    store.budget().release(65536);
}
