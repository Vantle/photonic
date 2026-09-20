use super::{Join, Request, Store, Traversal};
use crate::index::Index;
use crate::plan::Input;
use crate::program::Program;
use crate::state::State;
use std::sync::Arc;
use std::task::Poll;

fn compare(actual: &mut Join, expected: &mut Join, index: &Index, length: usize) -> bool {
    for _ in 0..length {
        let result = actual.step(index);
        assert_eq!(result, expected.step(index));
        assert_eq!(actual.retained(), actual.size());
        assert!(cached(actual) <= 4096);
        if result == Poll::Ready(None) {
            return true;
        }
    }
    false
}

fn cached(join: &Join) -> usize {
    match &join.traversal {
        Traversal::Factored(product) => product.cached(),
        Traversal::Partitioned(product) => product.cached(),
        _ => 0,
    }
}

#[test]
fn retraction() {
    let particle = ["A"; 8].join(".");
    let program = Program::new(
        crate::lowering::parse(&format!(
            "{particle},{particle},{particle},B,B,B,C,C,C [{particle},B,C] Done"
        ))
        .unwrap(),
    );
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let input = Input::new(&program.rule[0].input);
    let store = Arc::new(Store::new(65536));
    let mut actual = Join::planned(Request {
        input: &input,
        index: &index,
        frame: 0,
        owner: 0,
        store: &store,
    });
    let mut expected = Join::new(input.pattern(0), &index, 0);
    for iteration in 0..32 {
        assert!(compare(&mut actual, &mut expected, &index, 10000));
        let retained = cached(&actual);
        let removed = if iteration < 3 {
            state.world.len() - 1
        } else {
            state
                .world
                .iter()
                .position(|world| world.particle.len() == 8)
                .unwrap()
        };
        let mut world = (*state.world.remove(removed)).clone();
        for token in &mut world.particle {
            token.id += 1000;
        }
        state.world.push(world.into());
        index.advance(Arc::new(state.clone()), &crate::basis::Set::single(removed));
        actual.advance(&index);
        expected.advance(&index);
        if iteration > 5 {
            assert!(cached(&actual) > 0);
            assert!(cached(&actual) < retained);
        }
    }
    drop(actual);
    store.evict();
    assert!(store.budget().reserve(65536));
    store.budget().release(65536);
}

#[test]
fn mutation() {
    for capacity in [0, 2, 32, 128, 65536] {
        for particle in [["A"; 8].join("."), "A.B.C.D.E.F.G.H.([X] Y)".to_owned()] {
            let program = Program::new(
                crate::lowering::parse(&format!(
                    "{particle}.A,{particle}.A,{particle}.A,I,J,I,J,I,J [{particle},I,J] Done"
                ))
                .unwrap(),
            );
            let mut state = State::initial(&program);
            let template = state
                .world
                .iter()
                .find(|world| world.particle.len() > 1)
                .unwrap()
                .clone();
            let mut index = Index::new(Arc::new(state.clone()));
            let input = Input::new(&program.rule[0].input);
            let store = Arc::new(Store::new(capacity));
            let mut actual = Join::planned(Request {
                input: &input,
                index: &index,
                frame: 0,
                owner: 0,
                store: &store,
            });
            let mut expected = Join::new(input.pattern(0), &index, 0);
            for iteration in 0..96 {
                compare(&mut actual, &mut expected, &index, iteration % 43);
                if iteration % 13 == 0 {
                    actual.evict();
                    assert!(compare(&mut actual, &mut expected, &index, 100000));
                }
                actual.reset(&index);
                expected.reset(&index);
                assert!(compare(&mut actual, &mut expected, &index, 100000));
                let mut removed = crate::basis::Set::default();
                match iteration % 12 {
                    7 => {
                        state.world.push(template.clone());
                    }
                    8 => {
                        let position = state.world.len() - 1;
                        state.world.remove(position);
                        removed = crate::basis::Set::single(position);
                    }
                    _ => {
                        let position = if iteration % 12 == 9 {
                            state
                                .world
                                .iter()
                                .position(|world| world.particle.len() == 1)
                                .unwrap()
                        } else {
                            state
                                .world
                                .iter()
                                .position(|world| world.particle.len() > 1)
                                .unwrap()
                        };
                        let mut world = (*state.world.remove(position)).clone();
                        for token in &mut world.particle {
                            token.id += 1000;
                        }
                        state.world.push(world.into());
                        removed = crate::basis::Set::single(position);
                    }
                }
                index.advance(Arc::new(state.clone()), &removed);
                actual.advance(&index);
                expected.advance(&index);
            }
            drop(actual);
            store.evict();
            assert!(store.budget().reserve(capacity));
            store.budget().release(capacity);
        }
    }
}

#[test]
fn eviction() {
    let particle = ["A"; 8].join(".");
    let program = Program::new(
        crate::lowering::parse(&format!(
            "{particle}.A,{particle}.A,B,B,C,C [{particle},B,C] Done"
        ))
        .unwrap(),
    );
    let state = State::initial(&program);
    let index = Index::new(Arc::new(state));
    for offset in 0..128 {
        let input = Input::new(&program.rule[0].input);
        let store = Arc::new(Store::new(65536));
        let mut actual = Join::planned(Request {
            input: &input,
            index: &index,
            frame: 0,
            owner: 0,
            store: &store,
        });
        let mut expected = Join::new(input.pattern(0), &index, 0);
        actual.traversal =
            Join::product(&actual.space, &actual.order, super::Strategy::Partitioned).unwrap();
        assert!(compare(&mut actual, &mut expected, &index, 10000));
        assert!(cached(&actual) > 0);
        actual.reset(&index);
        expected.reset(&index);
        compare(&mut actual, &mut expected, &index, offset);
        actual.evict();
        assert_eq!(cached(&actual), 0);
        assert!(compare(&mut actual, &mut expected, &index, 10000));
        drop(actual);
        store.evict();
        assert!(store.budget().reserve(65536));
        store.budget().release(65536);
    }
}

#[test]
fn membership() {
    let particle = ["A"; 8].join(".");
    let program = Program::new(
        crate::lowering::parse(&format!(
            "{particle},{particle},B,B,B,B,B,C,C,C,C,C,C [{particle},B,C] Done"
        ))
        .unwrap(),
    );
    let mut state = State::initial(&program);
    let template = state.world[0].clone();
    let mut index = Index::new(Arc::new(state.clone()));
    let input = Input::new(&program.rule[0].input);
    let store = Arc::new(Store::new(65536));
    let mut actual = Join::planned(Request {
        input: &input,
        index: &index,
        frame: 0,
        owner: 0,
        store: &store,
    });
    let mut expected = Join::new(input.pattern(0), &index, 0);
    actual.traversal =
        Join::product(&actual.space, &actual.order, super::Strategy::Partitioned).unwrap();
    for _ in 0..16 {
        assert!(compare(&mut actual, &mut expected, &index, 10000));
        let retained = cached(&actual);
        assert!(retained > 0);
        state.world.push(template.clone());
        index.advance(Arc::new(state.clone()), &Default::default());
        actual.advance(&index);
        expected.advance(&index);
        assert_eq!(cached(&actual), retained);
        assert!(compare(&mut actual, &mut expected, &index, 10000));
        assert!(cached(&actual) > retained);
        let removed = state.world.len() - 1;
        state.world.remove(removed);
        index.advance(Arc::new(state.clone()), &crate::basis::Set::single(removed));
        actual.advance(&index);
        expected.advance(&index);
        assert_eq!(cached(&actual), retained);
    }
}

#[test]
fn saturation() {
    let particle = ["A"; 8].join(".");
    let pattern = ["B"; 13].join(",");
    let companion = ["B.C.E"; 14].join(",");
    for source in [
        format!("{particle},{particle},{companion},C [{particle},{pattern},B.E.E,C] Done"),
        format!("{particle}.A.A.A.A,{particle}.A.A.A.A,B,B,C,C [{particle},B,C] Done"),
    ] {
        let program = Program::new(crate::lowering::parse(&source).unwrap());
        let index = Index::new(Arc::new(State::initial(&program)));
        let input = Input::new(&program.rule[0].input);
        let store = Arc::new(Store::new(65536));
        let mut actual = Join::planned(Request {
            input: &input,
            index: &index,
            frame: 0,
            owner: 0,
            store: &store,
        });
        let mut expected = Join::new(input.pattern(0), &index, 0);
        actual.traversal =
            Join::product(&actual.space, &actual.order, super::Strategy::Partitioned).unwrap();
        for _ in 0..2 {
            assert!(compare(&mut actual, &mut expected, &index, 1000000));
            assert_eq!(cached(&actual), 0);
            actual.reset(&index);
            expected.reset(&index);
        }
        drop(actual);
        store.evict();
        assert!(store.budget().reserve(65536));
        store.budget().release(65536);
    }
}
