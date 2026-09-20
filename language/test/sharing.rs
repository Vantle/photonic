use super::product::Product;
use super::{Join, Request, Store, Traversal};
use crate::index::Index;
use crate::plan::Input;
use crate::program::Program;
use crate::state::State;
use std::sync::Arc;
use std::task::Poll;

fn prefix(join: &Join) -> &Product {
    let Traversal::Factored(product) = &join.traversal else {
        panic!(
            "expected factored traversal: {:?}, {:?}, {}",
            join.order,
            join.space.domain.iter().map(Vec::len).collect::<Vec<_>>(),
            join.stable
        );
    };
    product
}

fn drain(join: &mut Join, reference: &mut Join, index: &Index) {
    for _ in 0..100000 {
        let result = join.step(index);
        assert_eq!(result, reference.step(index));
        assert_eq!(join.retained(), join.size());
        if result == Poll::Ready(None) {
            return;
        }
    }
    panic!("expected complete traversal");
}

#[test]
fn projection() {
    for duplicate in [false, true] {
        let particle = ["A"; 8].join(".");
        let companion = if duplicate { particle.as_str() } else { "B" };
        let source = format!(
            "{particle}.A,{companion},C,D,C,D,C.D [C,{particle},{companion}] First [{particle},{companion},D] Second"
        );
        let program = Program::new(crate::lowering::parse(&source).unwrap());
        let mut state = State::initial(&program);
        let mut index = Index::new(Arc::new(state.clone()));
        let store = Arc::new(Store::new(65536));
        let input = program
            .rule
            .iter()
            .map(|rule| Input::new(&rule.input))
            .collect::<Vec<_>>();
        let mut query = input
            .iter()
            .map(|input| {
                Join::planned(Request {
                    input,
                    index: &index,
                    frame: 0,
                    owner: 0,
                    store: &store,
                })
            })
            .collect::<Vec<_>>();
        let mut reference = input
            .iter()
            .map(|input| Join::new(input.pattern(0), &index, 0))
            .collect::<Vec<_>>();
        for _ in 0..3 {
            let position = state
                .world
                .iter()
                .position(|world| {
                    world.particle.len() == 2
                        && world
                            .particle
                            .iter()
                            .all(|token| matches!(token.value, crate::program::Symbol::Atom(_)))
                })
                .unwrap();
            let world = state.world.remove(position);
            state.world.push(world);
            index.advance(
                Arc::new(state.clone()),
                &crate::basis::Set::single(position),
            );
            for (query, reference) in query.iter_mut().zip(&mut reference) {
                query.advance(&index);
                reference.advance(&index);
            }
        }
        for _ in 0..2 {
            drain(&mut query[0], &mut reference[0], &index);
            query[0].reset(&index);
            reference[0].reset(&index);
        }
        query[1].reset(&index);
        assert!(prefix(&query[0]).shares(prefix(&query[1])));
        assert_ne!(query[0].order, query[1].order);
        for position in 0..2 {
            drain(&mut query[position], &mut reference[position], &index);
            query[position].reset(&index);
            reference[position].reset(&index);
        }
        for iteration in 0..24 {
            let position = if iteration % 4 == 3 {
                0
            } else {
                state
                    .world
                    .iter()
                    .position(|world| world.particle.len() == 2)
                    .unwrap()
            };
            let world = state.world.remove(position);
            state.world.push(world);
            index.advance(
                Arc::new(state.clone()),
                &crate::basis::Set::single(position),
            );
            for (query, reference) in query.iter_mut().zip(&mut reference) {
                query.advance(&index);
                reference.advance(&index);
                drain(query, reference, &index);
                query.reset(&index);
                reference.reset(&index);
            }
        }
        for iteration in 0..96 {
            for position in 0..2 {
                assert_eq!(
                    query[position].step(&index),
                    reference[position].step(&index)
                );
            }
            if iteration == 3 {
                query[0].evict();
                store.evict();
            }
            if iteration == 7 {
                query[1].evict();
            }
        }
        for (query, reference) in query.iter_mut().zip(&mut reference) {
            drain(query, reference, &index);
        }
        drop(query);
        store.evict();
        assert_eq!(store.retained(), 0);
        assert!(store.budget().reserve(65536));
        store.budget().release(65536);
    }
}

#[test]
fn revision() {
    let program = Program::new(crate::lowering::parse("A,B [A] B").unwrap());
    let state = Arc::new(State::initial(&program));
    let mut left = Index::new(state.clone());
    let right = Index::new(state.clone());
    let previous = left.revision().clone();
    assert!(!Arc::ptr_eq(&previous, right.revision()));
    left.advance(state, &crate::basis::Set::default());
    assert!(!Arc::ptr_eq(&previous, left.revision()));
}

#[test]
fn isolation() {
    let program = Program::new(crate::lowering::parse("A").unwrap());
    let state = Arc::new(State::initial(&program));
    let mut index = Index::new(state.clone());
    let other = Index::new(state.clone());
    let store = Store::new(128);
    let mut node = Vec::new();
    for capture in [0, 1] {
        let pattern = Arc::new(vec![vec![crate::term::Term::new(
            crate::program::Symbol::Rule(0),
            Some(capture),
        )]]);
        let space = super::space::Space::new(pattern, &index, 0, None, None);
        node.push(store.subscribe(super::key::Key::new(&space, &[0])).unwrap());
    }
    assert!(!Arc::ptr_eq(&node[0], &node[1]));
    let mut trace = super::trace::Trace::new(store.budget().clone()).unwrap();
    assert!(trace.append(&Poll::Ready(None)));
    let trace = Arc::new(trace);
    node[0].publish(&index, &trace);
    assert!(Arc::ptr_eq(&node[0].find(&index).unwrap(), &trace));
    assert!(node[1].find(&index).is_none());
    assert!(node[0].find(&other).is_none());
    index.advance(state, &crate::basis::Set::single(0));
    assert!(node[0].find(&index).is_none());
    node[0].publish(&index, &trace);
    drop(trace);
    assert!(node[0].find(&index).is_none());
    drop(node);
    store.evict();
    assert_eq!(store.retained(), 0);
    assert!(store.budget().reserve(128));
    store.budget().release(128);
}

#[test]
fn pressure() {
    let program = Program::new(crate::lowering::parse("A").unwrap());
    let index = Index::new(Arc::new(State::initial(&program)));
    let store = Store::new(3);
    let space = |value| {
        super::space::Space::new(
            Arc::new(vec![vec![crate::term::Term::new(
                crate::program::Symbol::Atom(value),
                None,
            )]]),
            &index,
            0,
            None,
            None,
        )
    };
    let first = store
        .subscribe(super::key::Key::new(&space(0), &[0]))
        .unwrap();
    assert!(
        store
            .subscribe(super::key::Key::new(&space(1), &[0]))
            .is_none()
    );
    drop(first);
    let second = store
        .subscribe(super::key::Key::new(&space(1), &[0]))
        .unwrap();
    assert_eq!(store.retained(), 3);
    store.evict();
    assert_eq!(store.retained(), 3);
    drop(second);
    assert_eq!(store.retained(), 0);
    assert!(store.budget().reserve(3));
    store.budget().release(3);
}
