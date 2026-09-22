use super::Store;
use crate::program::Symbol;
use crate::state::{Canonical, Frame, State, Token, World};
use std::sync::Arc;

fn canonical() -> Canonical {
    State {
        world: vec![Arc::new(World {
            frame: 1,
            particle: vec![Token {
                id: 7,
                value: Symbol::Atom(0),
                capture: None,
            }],
        })]
        .into(),
        frame: vec![
            Arc::new(Frame {
                scope: 0,
                parent: None,
                lexical: None,
                held: Vec::new(),
            }),
            Arc::new(Frame {
                scope: 1,
                parent: Some(0),
                lexical: Some(0),
                held: vec![Token {
                    id: 42,
                    value: Symbol::Rule(0),
                    capture: Some(1),
                }],
            }),
        ]
        .into(),
    }
    .canonical()
}

#[test]
fn ownership() {
    let expected = canonical();
    let mut store = Store::default();
    let first = store.insert(canonical());
    let second = store.insert(canonical());
    assert_eq!(first.state, expected.state);
    assert_eq!(first.world, expected.world);
    assert_eq!(first.frame, expected.frame);
    assert_eq!(first.resource, expected.resource);
    for index in 0..first.state.frame.len() {
        assert!(Arc::ptr_eq(
            &first.state.frame[index],
            &second.state.frame[index]
        ));
    }
    let weak = Arc::downgrade(&first.state.frame[1]);
    drop(store);
    let mut changed = first.state.clone();
    Arc::make_mut(&mut changed.frame[1]).held[0].capture = Some(0);
    assert_ne!(changed, expected.state);
    assert_eq!(first.state, expected.state);
    assert_eq!(second.state, expected.state);
    drop(first);
    drop(second);
    assert!(weak.upgrade().is_none());
}

#[test]
fn capture() {
    let mut store = Store::default();
    let original = store.insert(canonical());
    let mut changed = canonical().state;
    Arc::make_mut(&mut changed.frame[1]).held[0].capture = Some(0);
    let changed = store.insert(changed.canonical());
    assert!(!Arc::ptr_eq(
        &original.state.frame[1],
        &changed.state.frame[1]
    ));
    assert_eq!(original.state.frame[1].held[0].capture, Some(1));
    assert_eq!(changed.state.frame[1].held[0].capture, Some(0));
    let mut independent = Store::default();
    let separate = independent.insert(canonical());
    assert!(!Arc::ptr_eq(
        &original.state.frame[1],
        &separate.state.frame[1]
    ));
}
