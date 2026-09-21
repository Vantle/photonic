use super::{Budget, Cursor};
use crate::particle::Match;
use crate::program::Symbol;
use crate::state::Token;
use crate::term::Term;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::task::Poll;

fn search() -> Match {
    Match::new(
        &vec![Term::new(Symbol::Atom(0), None); 8],
        &(0..12)
            .map(|id| Token {
                id,
                value: Symbol::Atom(0),
                capture: None,
            })
            .collect::<Vec<_>>(),
    )
}

#[test]
fn impossible() {
    let budget = Arc::new(Budget::new(8192));
    let mut cursor = Cursor::new(Match::impossible(), Some(budget.clone()));
    for _ in 0..4 {
        assert_eq!(cursor.step(8192), Poll::Ready(None));
        assert_eq!(cursor.cached(), 0);
        assert_eq!(budget.retained.load(Ordering::Relaxed), 0);
        cursor.reset();
    }
}

#[test]
fn continuation() {
    for capacity in [0, 1, 10, 64, 8192] {
        let budget = Arc::new(Budget::new(capacity));
        let mut cursor = Cursor::new(search(), Some(budget.clone()));
        let mut reference = search();
        for length in [0, 1, 7, 3, 13, 500, 9, 500] {
            for _ in 0..length {
                assert_eq!(cursor.step(8192 - cursor.cached()), reference.step());
                assert_eq!(budget.retained.load(Ordering::Relaxed), cursor.cached());
                assert!(cursor.cached() <= capacity);
            }
            cursor.reset();
            reference.reset();
        }
        drop(cursor);
        assert_eq!(budget.retained.load(Ordering::Relaxed), 0);
    }
}

#[test]
fn pressure() {
    let budget = Arc::new(Budget::new(128));
    let mut left = Cursor::new(search(), Some(budget.clone()));
    let mut right = Cursor::new(search(), Some(budget.clone()));
    let mut reference = search();
    for _ in 0..3 {
        for _ in 0..500 {
            let expected = reference.step();
            assert_eq!(left.step(64 - left.cached()), expected);
            assert_eq!(right.step(64 - right.cached()), expected);
            assert_eq!(
                budget.retained.load(Ordering::Relaxed),
                left.cached() + right.cached()
            );
            assert!(left.cached() <= 64 && right.cached() <= 64);
        }
        left.reset();
        right.reset();
        reference.reset();
    }
    assert_eq!(left.cached() + right.cached(), 0);
    assert_eq!(left.step(64), Poll::Ready(Some((0..8).collect())));
}

#[test]
fn eviction() {
    for length in [0, 1, 7, 49, 495, 496, 500] {
        let budget = Arc::new(Budget::new(8192));
        let mut cursor = Cursor::new(search(), Some(budget.clone()));
        for _ in 0..2 {
            while cursor.step(8192 - cursor.cached()) != Poll::Ready(None) {}
            cursor.reset();
        }
        assert!(cursor.cached() > 0);
        let mut reference = search();
        for _ in 0..length {
            assert_eq!(cursor.step(8192), reference.step());
        }
        cursor.evict();
        assert_eq!(cursor.cached(), 0);
        assert_eq!(budget.retained.load(Ordering::Relaxed), 0);
        for _ in 0..500 {
            assert_eq!(cursor.step(8192), reference.step());
        }
        cursor.reset();
        reference.reset();
        for _ in 0..500 {
            assert_eq!(cursor.step(8192), reference.step());
        }
    }
}
