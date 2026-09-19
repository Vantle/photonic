use super::Query;
use crate::basis::Set;
use crate::index::Index;
use crate::program::Program;
use crate::state::{State, Token, World};
use std::sync::Arc;

#[test]
fn sharing() {
    let program = Program::new(crate::lowering::parse("X,A [A] B [A] C").unwrap());
    let mut query = Query::new(&program);
    let initial = Arc::new(State::initial(&program));
    let mut index = Index::new(initial.clone());
    let first = query.select(0, 0, 0, &index);
    let second = query.select(1, 0, 0, &index);
    assert!(Arc::ptr_eq(&first, &second));
    let matched = program.rule[0].input[0][0];
    let removed = initial
        .world
        .iter()
        .position(|world| world.particle[0].value != matched)
        .unwrap();
    let mut state = (*initial).clone();
    state.world.remove(removed);
    index.advance(Arc::new(state.clone()), &[removed].into_iter().collect());
    query.advance(&index);
    let next = query.select(0, 0, 0, &index);
    assert!(Arc::ptr_eq(&first, &next));
    assert_eq!(index.world(next.candidate[0][0]), 0);
    index.advance(Arc::new(state.clone()), &[0].into_iter().collect());
    query.advance(&index);
    let changed = query.select(0, 0, 0, &index);
    assert!(!Arc::ptr_eq(&next, &changed));
    assert_eq!(query.preparation, 2);
    assert_eq!(query.reuse, 2);
    query.finish();
    assert_eq!(query.entry.len(), 1);
    query.advance(&index);
    query.finish();
    assert!(query.entry.is_empty());
}

#[test]
fn multiplicity() {
    let program = Program::new(crate::lowering::parse("A [A,A] B").unwrap());
    let mut query = Query::new(&program);
    let initial = Arc::new(State::initial(&program));
    let mut index = Index::new(initial.clone());
    let missing = query.select(0, 0, 0, &index);
    assert!(missing.candidate.is_empty());
    let mut state = (*initial).clone();
    state.world.push(Arc::new(World {
        frame: 0,
        particle: vec![Token {
            id: 100,
            value: program.rule[0].input[0][0],
            capture: None,
        }],
    }));
    index.advance(Arc::new(state), &Set::default());
    query.advance(&index);
    let present = query.select(0, 0, 0, &index);
    assert!(!present.candidate.is_empty());
    assert!(!Arc::ptr_eq(&missing, &present));
}

#[test]
fn empty() {
    let program = Program::new(crate::lowering::parse("[] A").unwrap());
    let mut query = Query::new(&program);
    let mut state = State::initial(&program);
    state.world.push(Arc::new(World {
        frame: 0,
        particle: Vec::new(),
    }));
    let mut index = Index::new(Arc::new(state.clone()));
    let present = query.select(0, 0, 0, &index);
    assert!(!present.candidate.is_empty());
    state.world.clear();
    index.advance(Arc::new(state), &[0].into_iter().collect());
    query.advance(&index);
    let missing = query.select(0, 0, 0, &index);
    assert!(missing.candidate.is_empty());
}
