use super::Store;
use crate::particle::Match;
use crate::program::{Program, Symbol};
use crate::state::{State, Token};
use crate::term::Term;
use std::sync::Arc;
use std::task::Poll;

#[test]
fn context() {
    let store = Store::new(256);
    let program = Program::new(crate::lowering::parse("A.([A] B)").unwrap());
    let mut state = State::initial(&program);
    let world = Arc::make_mut(&mut state.world[0]);
    world.particle = (0..40)
        .map(|id| Token {
            id,
            value: if id < 10 {
                Symbol::Rule(0)
            } else {
                Symbol::Atom(id)
            },
            capture: (id < 10).then_some(0),
        })
        .collect();
    let pattern = vec![Term::new(Symbol::Rule(0), Some(0)); 8];
    let compiled = store.compile(&pattern);
    assert!(Arc::ptr_eq(&compiled, &store.compile(&pattern)));
    let foreign = store.compile(&vec![Term::new(Symbol::Rule(0), Some(1)); 8]);
    assert!(!Arc::ptr_eq(&compiled, &foreign));
    assert_eq!(
        store.select(&foreign, &state.world[0]).step(),
        Poll::Ready(None)
    );
    let mut left = store.select(&compiled, &state.world[0]);
    let mut right = store.select(&compiled, &state.world[0]);
    assert!(Arc::ptr_eq(&left.preparation(), &right.preparation()));
    let mut expected = Match::new(&pattern, &state.world[0].particle);
    for _ in 0..3 {
        loop {
            let result = expected.step();
            assert_eq!(left.step(), result);
            store.evict();
            assert_eq!(store.retained(), 0);
            assert_eq!(right.step(), result);
            if matches!(result, Poll::Ready(None)) {
                break;
            }
        }
        left.reset();
        right.reset();
        expected.reset();
    }
    let previous = state.world[0].clone();
    for token in &mut Arc::make_mut(&mut state.world[0]).particle {
        token.id += 100;
    }
    assert_eq!(
        store.select(&compiled, &previous).step(),
        Poll::Ready(Some((0..8).collect()))
    );
    assert_eq!(
        store.select(&compiled, &state.world[0]).step(),
        Poll::Ready(Some((100..108).collect()))
    );
}

#[test]
fn pressure() {
    let program = Program::new(crate::lowering::parse("A.B.C.D.E.F.G.H,X,Y").unwrap());
    let mut state = State::initial(&program);
    Arc::make_mut(&mut state.world[0])
        .particle
        .extend((0..32).map(|id| Token {
            id: 100 + id,
            value: Symbol::Atom(100),
            capture: None,
        }));
    let pattern = state.world[0].particle[..8]
        .iter()
        .map(|token| Term::new(token.value, token.capture))
        .collect::<Vec<_>>();
    for capacity in [0, 1, 32, 128, 4096] {
        let store = Arc::new(Store::new(capacity));
        let index = Arc::new(crate::index::Index::new(Arc::new(state.clone())));
        let mut search = (0..8)
            .map(|iteration| {
                let mut pattern = vec![pattern.clone()];
                pattern.push(vec![Term::new(
                    state.world[1 + iteration % 2].particle[0].value,
                    None,
                )]);
                (
                    crate::search::Search::shared(pattern.clone(), index.clone(), 0, &store),
                    crate::search::Search::new(pattern, index.clone(), 0),
                )
            })
            .collect::<Vec<_>>();
        for step in 0..100 {
            for (actual, expected) in &mut search {
                assert_eq!(actual.step(), expected.step());
                assert_eq!(actual.retained(), expected.retained());
                assert!(store.retained() <= capacity);
            }
            if step % 7 == 0 {
                store.evict();
            }
        }
        store.evict();
        assert_eq!(store.retained(), 0);
    }
}

#[test]
fn concurrent() {
    let store = Store::new(4096);
    let world = Arc::new(crate::state::World {
        frame: 0,
        particle: (0..40)
            .map(|id| Token {
                id,
                value: Symbol::Atom(id),
                capture: None,
            })
            .collect(),
    });
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| {
                for _ in 0..128 {
                    let pattern = (0..8)
                        .map(|id| Term::new(Symbol::Atom(id), None))
                        .collect::<Vec<_>>();
                    let compiled = store.compile(&pattern);
                    let mut search = store.select(&compiled, &world);
                    assert_eq!(search.step(), Poll::Ready(Some((0..8).collect())));
                    assert_eq!(search.step(), Poll::Ready(None));
                }
            });
        }
    });
    assert!(store.retained() <= 4096);
    store.evict();
    assert_eq!(store.retained(), 0);
}
