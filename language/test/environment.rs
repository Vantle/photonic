use super::{Request, Store};
use crate::state::{Frame, State, Token};
use std::sync::Arc;

fn state() -> State {
    State {
        world: Default::default(),
        frame: (0..2)
            .map(|value| {
                Arc::new(Frame {
                    scope: 0,
                    parent: None,
                    lexical: None,
                    held: vec![Token {
                        id: value,
                        value: crate::program::Symbol::Atom(value),
                        capture: None,
                    }],
                })
            })
            .collect(),
    }
}

#[test]
fn identity() {
    let state = state();
    let mut store = Store::default();
    let first = store.resolve(Request {
        target: 0,
        capture: 0,
        state: &state,
    });
    let repeated = store.resolve(Request {
        target: 0,
        capture: 0,
        state: &state,
    });
    let target = store.resolve(Request {
        target: 1,
        capture: 0,
        state: &state,
    });
    let capture = store.resolve(Request {
        target: 0,
        capture: 1,
        state: &state,
    });
    assert!(Arc::ptr_eq(&first, &repeated));
    assert!(!Arc::ptr_eq(&first, &target));
    assert_eq!(first, target);
    assert_ne!(first, capture);
    assert_eq!(*first, state.environment(0));
    assert_eq!(*capture, state.environment(1));
    let weak = Arc::downgrade(&first);
    drop(first);
    drop(repeated);
    assert!(weak.upgrade().is_none());
    let replacement = store.resolve(Request {
        target: 0,
        capture: 0,
        state: &state,
    });
    assert_eq!(replacement, target);
    assert!(!std::ptr::eq(weak.as_ptr(), Arc::as_ptr(&replacement)));
}

#[test]
fn capacity() {
    for capacity in [0, 1, 2] {
        let state = state();
        let mut store = Store::new(capacity);
        let mut retained = Vec::new();
        for target in 0..8 {
            let environment = store.resolve(Request {
                target,
                capture: target % 2,
                state: &state,
            });
            assert_eq!(*environment, state.environment(target % 2));
            retained.push(environment);
            assert!(store.entry.len() <= capacity);
        }
        drop(retained);
        let environment = store.resolve(Request {
            target: 8,
            capture: 0,
            state: &state,
        });
        assert_eq!(*environment, state.environment(0));
        assert_eq!(store.entry.len(), usize::from(capacity != 0));
    }
}
