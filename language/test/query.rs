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
    assert!(Arc::ptr_eq(&next, &changed));
    assert_eq!(query.preparation, 1);
    assert_eq!(query.reuse, 3);
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
    assert!(!missing.viable);
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
    assert!(present.viable);
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
    assert!(present.viable);
    state.world.clear();
    index.advance(Arc::new(state), &[0].into_iter().collect());
    query.advance(&index);
    let missing = query.select(0, 0, 0, &index);
    assert!(!missing.viable);
}

#[test]
fn capture() {
    use crate::program::Symbol;
    let mut program = Program::new(crate::lowering::parse("A [A] B [B] C").unwrap());
    program.rule[0].input = vec![vec![Symbol::Rule(1)]];
    let mut state = State::initial(&program);
    state.frame.push(state.frame[0].clone());
    state.world = vec![Arc::new(World {
        frame: 0,
        particle: vec![Token {
            id: 0,
            value: Symbol::Rule(1),
            capture: Some(0),
        }],
    })];
    let mut query = Query::new(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    assert!(query.select(0, 0, 0, &index).viable);
    assert!(!query.select(0, 0, 1, &index).viable);
    Arc::make_mut(&mut state.world[0]).particle[0].capture = Some(1);
    index.advance(Arc::new(state), &Set::single(0));
    query.advance(&index);
    assert!(!query.select(0, 0, 0, &index).viable);
    assert!(query.select(0, 0, 1, &index).viable);
}

#[test]
fn ownership() {
    let program = Program::new(crate::lowering::parse("A [A] B").unwrap());
    let mut state = State::initial(&program);
    state.frame.push(state.frame[0].clone());
    let mut query = Query::new(&program);
    let index = Index::new(Arc::new(state));
    let first = query.select(0, 0, 0, &index);
    let second = query.select(0, 0, 1, &index);
    assert!(Arc::ptr_eq(&first, &second));
}
