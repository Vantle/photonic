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
        assert!(actual.cached <= 4096);
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
            let program = Program::new(crate::lowering::parse(&source).unwrap());
            let mut state = State::initial(&program);
            let mut index = Index::new(Arc::new(state.clone()));
            let input = crate::plan::Input::new(&program.rule[0].input);
            let budget = Arc::new(crate::factor::Budget::new(capacity));
            let mut actual = Join::planned(&input, &index, 0, 0, &budget);
            let mut expected = Join::new(input.pattern(0), &index, 0);
            for iteration in 0..96 {
                compare(&mut actual, &mut expected, &index, iteration % 31);
                actual.reset();
                expected.reset();
                compare(&mut actual, &mut expected, &index, 10000);
                let removed = iteration % state.world.len();
                let world = state.world.remove(removed);
                state.world.push(world);
                index.advance(Arc::new(state.clone()), &crate::basis::Set::single(removed));
                actual.advance(&index);
                expected.advance(&index);
                compare(&mut actual, &mut expected, &index, 10000);
                actual.reset();
                expected.reset();
            }
        }
    }
}

#[test]
fn survival() {
    let value = ["A"; 8].join(".");
    let source = format!("{value},B [{value},B] Done");
    let program = Program::new(crate::lowering::parse(&source).unwrap());
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let input = crate::plan::Input::new(&program.rule[0].input);
    let budget = Arc::new(crate::factor::Budget::new(65536));
    let mut actual = Join::planned(&input, &index, 0, 0, &budget);
    let mut expected = Join::new(input.pattern(0), &index, 0);
    for _ in 0..3 {
        compare(&mut actual, &mut expected, &index, 100);
        actual.reset();
        expected.reset();
    }
    assert!(actual.cached > 0);
    let retained = actual.cached;
    let world = state.world.remove(1);
    state.world.push(world);
    index.advance(Arc::new(state.clone()), &crate::basis::Set::single(1));
    actual.advance(&index);
    expected.advance(&index);
    assert_eq!(actual.cached, retained);
    compare(&mut actual, &mut expected, &index, 100);
    let world = state.world.remove(0);
    state.world.push(world);
    index.advance(Arc::new(state), &crate::basis::Set::single(0));
    actual.advance(&index);
    expected.advance(&index);
    assert_eq!(actual.cached, 0);
    compare(&mut actual, &mut expected, &index, 100);
}
