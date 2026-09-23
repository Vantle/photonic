use super::product::Product;
use super::{Join, Request, Store, Traversal};
use crate::index::Index;
use crate::plan::Input;
use crate::program::Program;
use crate::state::State;
use std::sync::Arc;
use std::task::Poll;

fn prefix(join: &Join) -> &Product<super::stream::Stream> {
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
        let program = Program::new(&crate::lowering::parse(&source).unwrap());
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
    let program = Program::new(&crate::lowering::parse("A,B [A] B").unwrap());
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
    let program = Program::new(&crate::lowering::parse("A").unwrap());
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
        let space = super::space::Space::new(super::query::Query::direct(pattern), &index, 0, None);
        node.push(store.subscribe(super::key::Key::new(&space, &[0])).unwrap());
    }
    assert!(!Arc::ptr_eq(&node[0], &node[1]));
    let mut trace = super::trace::Trace::new(store.budget().clone(), 1).unwrap();
    assert!(trace.append(&Poll::Ready(None), 0.., 4096));
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
    let program = Program::new(&crate::lowering::parse("A").unwrap());
    let index = Index::new(Arc::new(State::initial(&program)));
    let store = Store::new(3);
    let space = |value| {
        super::space::Space::new(
            super::query::Query::direct(Arc::new(vec![vec![crate::term::Term::new(
                crate::program::Symbol::Atom(value),
                None,
            )]])),
            &index,
            0,
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

fn advance(query: &mut Join, reference: &mut Join, index: &Index, length: usize) {
    for _ in 0..length {
        let result = query.step(index);
        assert_eq!(result, reference.step(index));
        assert_eq!(query.retained(), query.size());
        assert!(prefix(query).cached() <= 4096);
        if result == Poll::Ready(None) {
            return;
        }
    }
}

#[test]
fn unfinished() {
    for productive in [false, true] {
        let particle = format!("{}.([X] Y)", ["A"; 8].join("."));
        let source = if productive {
            format!(
                "{particle}.A,I,I,I,I,J,J,J,J,J,K.L.M,K.L.M,K.L.M,K.L.M,K.L.M,K.L.M [{particle},I,J,K] First [L,{particle},I,J] Second [{particle},I,J,M] Third"
            )
        } else {
            let content = ["I.J.K.L.M"; 10].join(",");
            let pattern = ["I"; 9].join(",");
            format!(
                "{particle},{content},K.L.M [{particle},{pattern},I.J.J,K] First [L,{particle},{pattern},I.J.J] Second [{particle},{pattern},I.J.J,M] Third"
            )
        };
        let program = Program::new(&crate::lowering::parse(&source).unwrap());
        let index = Index::new(Arc::new(State::initial(&program)));
        let store = Arc::new(Store::new(65536));
        let input = program
            .rule
            .iter()
            .filter(|rule| rule.input.len() >= 3)
            .map(|rule| Input::new(&rule.input))
            .collect::<Vec<_>>();
        let mut query = input
            .iter()
            .map(|input| {
                let mut query = Join::planned(Request {
                    input,
                    index: &index,
                    frame: 0,
                    owner: 0,
                    store: &store,
                });
                query.traversal =
                    Join::product(&query.space, &query.order, super::Strategy::Shared).unwrap();
                query
            })
            .collect::<Vec<_>>();
        let mut reference = input
            .iter()
            .map(|input| Join::new(input.pattern(0), &index, 0))
            .collect::<Vec<_>>();
        let node = store
            .subscribe(super::key::Key::new(
                &query[0].space,
                &query[0].order[..query[0].order.len() - 1],
            ))
            .unwrap();
        drain(&mut query[0], &mut reference[0], &index);
        query[0].reset(&index);
        reference[0].reset(&index);
        for _ in 0..100000 {
            advance(&mut query[0], &mut reference[0], &index, 1);
            if node.find(&index).is_some() {
                break;
            }
        }
        let snapshot = node.find(&index).unwrap();
        assert!(!snapshot.complete);
        assert_eq!(snapshot.length, 256);
        drop(snapshot);
        for query in &mut query[1..] {
            query.reset(&index);
        }
        assert!(prefix(&query[1]).shares(prefix(&query[2])));
        advance(&mut query[1], &mut reference[1], &index, 4000);
        advance(&mut query[2], &mut reference[2], &index, 97);
        if productive {
            drain(&mut query[0], &mut reference[0], &index);
            assert!(node.find(&index).unwrap().complete);
        } else {
            advance(&mut query[0], &mut reference[0], &index, 2048);
            assert!(node.find(&index).unwrap().length >= 2048);
        }
        advance(&mut query[2], &mut reference[2], &index, 1200);
        query.remove(0);
        reference.remove(0);
        query[1].evict();
        store.evict();
        for (query, reference) in query.iter_mut().zip(&mut reference) {
            drain(query, reference, &index);
        }
        drop(query);
        drop(node);
        store.evict();
        assert!(store.budget().reserve(65536));
        store.budget().release(65536);
    }
}

#[test]
fn snapshot() {
    let budget = Arc::new(crate::budget::Budget::new(65536));
    let mut trace = super::trace::Trace::new(budget.clone(), 1).unwrap();
    for length in [3, 1, 5, 2] {
        for _ in 0..length {
            assert!(trace.append(&Poll::Pending, 0.., 4096));
        }
        assert!(trace.append(
            &Poll::Ready(Some(vec![super::slot::Slot {
                site: length,
                position: 0,
                token: vec![length, length + 1]
            }])),
            0..,
            4096
        ));
    }
    assert!(trace.duplicate(trace.retained - 1).is_none());
    let snapshot = trace.duplicate(trace.retained).unwrap();
    assert!(trace.append(&Poll::Pending, 0.., 4096));
    assert_eq!(snapshot.length + 1, trace.length);
    for offset in 0..=snapshot.length {
        let mut actual = super::playback::Playback::default();
        let mut expected = super::playback::Playback::default();
        actual.seek(&snapshot, offset);
        for _ in 0..offset {
            expected.step(&snapshot, &[0], &[]);
        }
        loop {
            let result = actual.step(&snapshot, &[0], &[]);
            assert_eq!(result, expected.step(&snapshot, &[0], &[]));
            if result.is_none() {
                break;
            }
        }
    }
    drop(trace);
    drop(snapshot);
    assert!(budget.reserve(65536));
    budget.release(65536);
}

#[test]
fn mutation() {
    let particle = format!("{}.([X] Y)", ["A"; 8].join("."));
    let content = ["I.J.K.L.M"; 8].join(",");
    let pattern = ["I"; 7].join(",");
    let program = Program::new(&crate::lowering::parse(&format!(
        "{particle},{content},K.L.M [{particle},{pattern},I.J.J,K] First [L,{particle},{pattern},I.J.J] Second [{particle},{pattern},I.J.J,M] Third"
    )).unwrap());
    for capacity in [0, 32, 128, 512, 65536] {
        for replacement in [false, true] {
            for reverse in [false, true] {
                let mut state = State::initial(&program);
                let mut index = Index::new(Arc::new(state.clone()));
                let store = Arc::new(Store::new(capacity));
                let input = program
                    .rule
                    .iter()
                    .filter(|rule| rule.input.len() >= 3)
                    .map(|rule| Input::new(&rule.input))
                    .collect::<Vec<_>>();
                let mut query = input
                    .iter()
                    .map(|input| {
                        let mut query = Join::planned(Request {
                            input,
                            index: &index,
                            frame: 0,
                            owner: 0,
                            store: &store,
                        });
                        query.traversal =
                            Join::product(&query.space, &query.order, super::Strategy::Shared)
                                .unwrap();
                        query
                    })
                    .collect::<Vec<_>>();
                let mut reference = input
                    .iter()
                    .map(|input| Join::new(input.pattern(0), &index, 0))
                    .collect::<Vec<_>>();
                drain(&mut query[0], &mut reference[0], &index);
                query[0].reset(&index);
                reference[0].reset(&index);
                advance(&mut query[0], &mut reference[0], &index, 600);
                for query in &mut query[1..] {
                    query.reset(&index);
                }
                advance(&mut query[1], &mut reference[1], &index, 97);
                query[0].reset(&index);
                reference[0].reset(&index);
                advance(&mut query[0], &mut reference[0], &index, 900);
                let position = state
                    .world
                    .iter()
                    .position(|world| world.particle.len() == if replacement { 9 } else { 3 })
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
                let order = if reverse { [2, 1, 0] } else { [0, 1, 2] };
                for position in order {
                    query[position].advance(&index);
                    reference[position].advance(&index);
                }
                for position in order {
                    drain(&mut query[position], &mut reference[position], &index);
                    query[position].reset(&index);
                    reference[position].reset(&index);
                }
                query[1].evict();
                store.evict();
                for position in order {
                    drain(&mut query[position], &mut reference[position], &index);
                }
                drop(query);
                store.evict();
                assert!(store.budget().reserve(capacity));
                store.budget().release(capacity);
            }
        }
    }
}
