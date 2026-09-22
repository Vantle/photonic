use super::Join;
use crate::index::Index;
use crate::program::Program;
use crate::state::State;
use std::sync::Arc;
use std::task::Poll;

fn compare(actual: &mut Join, expected: &mut Join, index: &Index, length: usize) {
    for _ in 0..length {
        let result = actual.step(index);
        assert_eq!(result, expected.step(index));
        assert_eq!(actual.retained(), actual.size());
        assert!(actual.space.cached <= 4096);
        if result == Poll::Ready(None) {
            break;
        }
    }
}

#[test]
fn mutation() {
    for capacity in [0, 32, 65536] {
        for repeated in [false, true] {
            let value = if repeated {
                vec!["A".to_owned(); 8]
            } else {
                (0..8).map(|index| format!("A{index}")).collect()
            }
            .join(".");
            let source = format!("{value}.A.A,{value}.B,B,B [{value},B] Done");
            let program = Program::new(&crate::lowering::parse(&source).unwrap());
            let mut state = State::initial(&program);
            let mut index = Index::new(Arc::new(state.clone()));
            let input = crate::plan::Input::new(&program.rule[0].input);
            let store = Arc::new(crate::joining::Store::new(capacity));
            let mut actual = Join::planned(super::Request {
                input: &input,
                index: &index,
                frame: 0,
                owner: 0,
                store: &store,
            });
            let mut expected = Join::new(input.pattern(0), &index, 0);
            for iteration in 0..96 {
                compare(&mut actual, &mut expected, &index, iteration % 31);
                actual.reset(&index);
                expected.reset(&index);
                compare(&mut actual, &mut expected, &index, 10000);
                let removed = iteration % state.world.len();
                let world = state.world.remove(removed);
                state.world.push(world);
                index.advance(Arc::new(state.clone()), &crate::basis::Set::single(removed));
                actual.advance(&index);
                expected.advance(&index);
                compare(&mut actual, &mut expected, &index, 10000);
                actual.reset(&index);
                expected.reset(&index);
            }
        }
    }
}

#[test]
fn survival() {
    let value = ["A"; 8].join(".");
    let source = format!("{value},B [{value},B] Done");
    let program = Program::new(&crate::lowering::parse(&source).unwrap());
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let input = crate::plan::Input::new(&program.rule[0].input);
    let store = Arc::new(crate::joining::Store::new(65536));
    let mut actual = Join::planned(super::Request {
        input: &input,
        index: &index,
        frame: 0,
        owner: 0,
        store: &store,
    });
    let mut expected = Join::new(input.pattern(0), &index, 0);
    for _ in 0..3 {
        compare(&mut actual, &mut expected, &index, 100);
        actual.reset(&index);
        expected.reset(&index);
    }
    assert!(actual.space.cached > 0);
    let retained = actual.space.cached;
    let world = state.world.remove(1);
    state.world.push(world);
    index.advance(Arc::new(state.clone()), &crate::basis::Set::single(1));
    actual.advance(&index);
    expected.advance(&index);
    assert_eq!(actual.space.cached, retained);
    compare(&mut actual, &mut expected, &index, 100);
    let world = state.world.remove(0);
    state.world.push(world);
    index.advance(Arc::new(state), &crate::basis::Set::single(0));
    actual.advance(&index);
    expected.advance(&index);
    assert_eq!(actual.space.cached, 0);
    compare(&mut actual, &mut expected, &index, 100);
}

#[test]
fn locality() {
    for pattern in ["A.B", "A", ""] {
        let source = format!("{},A.C [{pattern},C] Done", vec!["A.B"; 32].join(","));
        let program = Program::new(&crate::lowering::parse(&source).unwrap());
        let mut state = State::initial(&program);
        let mut index = Index::new(Arc::new(state.clone()));
        let input = crate::plan::Input::new(&program.rule[0].input);
        let store = Arc::new(crate::joining::Store::new(65536));
        let mut actual = Join::planned(super::Request {
            input: &input,
            index: &index,
            frame: 0,
            owner: 0,
            store: &store,
        });
        let mut expected = Join::new(input.pattern(0), &index, 0);
        for iteration in 0..128 {
            compare(&mut actual, &mut expected, &index, iteration % 17);
            let removed = if iteration % 3 == 0 {
                iteration % state.world.len()
            } else {
                state.world.len() - 1
            };
            let world = state.world.remove(removed);
            state.world.push(world);
            index.advance(Arc::new(state.clone()), &crate::basis::Set::single(removed));
            actual.advance(&index);
            expected.advance(&index);
            compare(&mut actual, &mut expected, &index, 10000);
            actual.reset(&index);
            expected.reset(&index);
        }
    }
}

#[test]
fn prefix() {
    let particle = ["A"; 8].join(".");
    for capacity in [0, 32, 128, 65536] {
        for source in [
            format!("{particle}.A,B,C [{particle},B,C] Done"),
            format!("{particle},B,C,C [{particle},B,C] Done"),
            format!("{particle},{particle},{particle},C [{particle},{particle},C] Done"),
            format!("{particle},B,C,D [{particle},B,C,D] Done"),
            format!("{particle},B,C [{particle},,C] Done"),
            format!("{particle}.([X] Y),B,C [{particle}.([X] Y),B,C] Done"),
        ] {
            let program = Program::new(&crate::lowering::parse(&source).unwrap());
            let mut state = State::initial(&program);
            let mut index = Index::new(Arc::new(state.clone()));
            let input = crate::plan::Input::new(&program.rule[0].input);
            let store = Arc::new(crate::joining::Store::new(capacity));
            let mut actual = Join::planned(super::Request {
                input: &input,
                index: &index,
                frame: 0,
                owner: 0,
                store: &store,
            });
            let mut expected = Join::new(input.pattern(0), &index, 0);
            for iteration in 0..48 {
                compare(&mut actual, &mut expected, &index, iteration);
                if iteration % 5 == 0 {
                    actual.evict();
                    compare(&mut actual, &mut expected, &index, 7);
                }
                actual.reset(&index);
                expected.reset(&index);
                compare(&mut actual, &mut expected, &index, 10000);
                let removed = if iteration % 4 == 0 {
                    iteration % state.world.len()
                } else {
                    state.world.len() - 1
                };
                let world = state.world.remove(removed);
                state.world.push(world);
                index.advance(Arc::new(state.clone()), &crate::basis::Set::single(removed));
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
fn saturation() {
    let particle = ["A"; 8].join(".");
    let source = format!("{particle}.A.A.A.A,B,C [{particle},B,C] Done");
    let program = Program::new(&crate::lowering::parse(&source).unwrap());
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let input = crate::plan::Input::new(&program.rule[0].input);
    let store = Arc::new(crate::joining::Store::new(65536));
    let mut actual = Join::planned(super::Request {
        input: &input,
        index: &index,
        frame: 0,
        owner: 0,
        store: &store,
    });
    let mut expected = Join::new(input.pattern(0), &index, 0);
    for _ in 0..3 {
        let world = state.world.remove(2);
        state.world.push(world);
        index.advance(Arc::new(state.clone()), &crate::basis::Set::single(2));
        actual.advance(&index);
        expected.advance(&index);
    }
    assert!(matches!(actual.traversal, super::Traversal::Factored(_)));
    for _ in 0..3 {
        compare(&mut actual, &mut expected, &index, 100000);
        actual.reset(&index);
        expected.reset(&index);
    }
    actual.evict();
    compare(&mut actual, &mut expected, &index, 100000);
}

#[test]
fn persistence() {
    let particle = ["A"; 8].join(".");
    let source = format!("{particle},B,C [{particle},B,C] Done");
    let program = Program::new(&crate::lowering::parse(&source).unwrap());
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let input = crate::plan::Input::new(&program.rule[0].input);
    let store = Arc::new(crate::joining::Store::new(65536));
    let mut actual = Join::planned(super::Request {
        input: &input,
        index: &index,
        frame: 0,
        owner: 0,
        store: &store,
    });
    let mut expected = Join::new(input.pattern(0), &index, 0);
    let cached = |join: &Join| match &join.traversal {
        super::Traversal::Factored(product) => product.cached(),
        _ => 0,
    };
    for _ in 0..5 {
        compare(&mut actual, &mut expected, &index, 100);
        let world = state.world.remove(2);
        state.world.push(world);
        index.advance(Arc::new(state.clone()), &crate::basis::Set::single(2));
        actual.advance(&index);
        expected.advance(&index);
    }
    let retained = cached(&actual);
    assert!(retained > 0);
    for _ in 0..32 {
        let world = state.world.remove(2);
        state.world.push(world);
        index.advance(Arc::new(state.clone()), &crate::basis::Set::single(2));
        actual.advance(&index);
        expected.advance(&index);
        assert_eq!(cached(&actual), retained);
        compare(&mut actual, &mut expected, &index, 100);
    }
    actual.reset(&index);
    expected.reset(&index);
    compare(&mut actual, &mut expected, &index, 3);
    actual.evict();
    assert_eq!(cached(&actual), 0);
    compare(&mut actual, &mut expected, &index, 100);
    let world = state.world.remove(0);
    state.world.push(world);
    index.advance(Arc::new(state), &crate::basis::Set::single(0));
    actual.advance(&index);
    expected.advance(&index);
    assert_eq!(cached(&actual), 0);
    compare(&mut actual, &mut expected, &index, 100);
}

#[test]
fn reordering() {
    let particle = ["A"; 8].join(".");
    let source = format!("{particle}.A,B,C [{particle},B,C] Done");
    let program = Program::new(&crate::lowering::parse(&source).unwrap());
    let mut state = State::initial(&program);
    let template = [state.world[1].clone(), state.world[2].clone()];
    let mut index = Index::new(Arc::new(state.clone()));
    let input = crate::plan::Input::new(&program.rule[0].input);
    let store = Arc::new(crate::joining::Store::new(65536));
    let mut actual = Join::planned(super::Request {
        input: &input,
        index: &index,
        frame: 0,
        owner: 0,
        store: &store,
    });
    let mut expected = Join::new(input.pattern(0), &index, 0);
    for iteration in 0..64 {
        compare(&mut actual, &mut expected, &index, iteration % 17);
        if iteration % 11 == 0 {
            actual.evict();
        }
        let mut removed = crate::basis::Set::default();
        match iteration % 8 {
            0 | 1 | 7 => {
                let position = if iteration % 8 == 7 {
                    0
                } else {
                    state.world.len() - 1
                };
                let world = state.world.remove(position);
                state.world.push(world);
                removed = crate::basis::Set::single(position);
            }
            2 | 4 => {
                state.world.push(template[0].clone());
            }
            3 | 5 => {
                let symbol = template[usize::from(iteration % 8 == 5)].particle[0].value;
                removed = state
                    .world
                    .iter()
                    .enumerate()
                    .filter(|(_, world)| world.particle[0].value == symbol)
                    .map(|(position, _)| position)
                    .collect();
                for &position in removed.iter().rev() {
                    state.world.remove(position);
                }
            }
            _ => {
                state.world.push(template[1].clone());
            }
        }
        index.advance(Arc::new(state.clone()), &removed);
        actual.advance(&index);
        expected.advance(&index);
        compare(&mut actual, &mut expected, &index, 10000);
        actual.reset(&index);
        expected.reset(&index);
    }
}

#[test]
fn frontier() {
    let particle = ["A"; 8].join(".");
    let content = ["B.C.E"; 13].join(",");
    let pattern = ["B"; 12].join(",");
    let source = format!("{particle},{content},C [{particle},{pattern},B.E.E,C] Never");
    let program = Program::new(&crate::lowering::parse(&source).unwrap());
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let input = crate::plan::Input::new(&program.rule[0].input);
    let store = Arc::new(crate::joining::Store::new(65536));
    let mut actual = Join::planned(super::Request {
        input: &input,
        index: &index,
        frame: 0,
        owner: 0,
        store: &store,
    });
    let mut expected = Join::new(input.pattern(0), &index, 0);
    for _ in 0..3 {
        let position = state.world.len() - 1;
        let world = state.world.remove(position);
        state.world.push(world);
        index.advance(
            Arc::new(state.clone()),
            &crate::basis::Set::single(position),
        );
        actual.advance(&index);
        expected.advance(&index);
    }
    for _ in 0..3 {
        compare(&mut actual, &mut expected, &index, 1000000);
        let super::Traversal::Factored(product) = &actual.traversal else {
            panic!("expected prefix admission");
        };
        assert_eq!(product.cached(), 0);
        actual.reset(&index);
        expected.reset(&index);
    }
    actual.evict();
    compare(&mut actual, &mut expected, &index, 1000000);
}
