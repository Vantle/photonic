use super::{Mode, Search};
use crate::index::Index;
use crate::joining::{Join, Request, Store};
use crate::plan::Input;
use crate::program::{Program, Symbol};
use crate::state::{Token, World};
use std::sync::Arc;
use std::task::Poll;

fn compare(actual: &mut Search, expected: &mut Join, index: &Index, length: usize) -> bool {
    for _ in 0..length {
        assert_eq!(actual.viable(), expected.viable());
        assert_eq!(actual.frame(), expected.frame());
        let skipped = actual.skip(3);
        for _ in 0..skipped {
            assert_eq!(expected.step(index), Poll::Pending);
        }
        let result = actual.step(index);
        assert_eq!(result, expected.step(index));
        if result == Poll::Ready(None) {
            return true;
        }
    }
    false
}

#[test]
fn activation() {
    for source in [
        "[] Done",
        "[,] Done",
        "[,,] Done",
        "[A] Done",
        "[A,B] Done",
        "[A,,B] Done",
        "[A,A] Done",
        "[A.A.A.A.A.A.A.A,B,C] Done",
        "[[A] B] Done",
    ] {
        for owner in 0..2 {
            let program = Program::new(&crate::lowering::parse(source).unwrap());
            let input = Input::new(&program.rule[0].input);
            let mut state = crate::state::State::initial(&program);
            state.frame.push(state.frame[0].clone());
            let mut index = Index::new(Arc::new(state.clone()));
            let store = Arc::new(Store::new(65536));
            let reference = Arc::new(Store::new(65536));
            let mut actual = Search::planned(Request {
                input: &input,
                index: &index,
                frame: 0,
                owner,
                store: &store,
            });
            let mut expected = Join::planned(Request {
                input: &input,
                index: &index,
                frame: 0,
                owner,
                store: &reference,
            });
            assert!(matches!(actual.mode, Mode::Dormant));
            let mut identity = 0;
            for iteration in 0..96 {
                compare(&mut actual, &mut expected, &index, iteration % 19);
                actual.reset(&index);
                expected.reset(&index);
                assert!(compare(&mut actual, &mut expected, &index, 10000));
                if iteration % 3 == 0 {
                    actual.evict();
                    expected.evict();
                    store.evict();
                    reference.evict();
                }
                let removal =
                    if !state.world.is_empty() && (iteration % 5 == 4 || state.world.len() >= 6) {
                        state.world.remove(0);
                        crate::basis::Set::single(0)
                    } else {
                        crate::basis::Set::default()
                    };
                if iteration % 4 != 3 {
                    let pattern = &program.rule[0].input;
                    let particle = if pattern.is_empty() {
                        Vec::new()
                    } else {
                        pattern[iteration % pattern.len()]
                            .iter()
                            .map(|&value| {
                                let token = Token {
                                    id: identity,
                                    value,
                                    capture: matches!(value, Symbol::Rule(_))
                                        .then_some(iteration % 2),
                                };
                                identity += 1;
                                token
                            })
                            .collect()
                    };
                    state.world.push(
                        World {
                            frame: usize::from(iteration % 7 == 0),
                            particle,
                        }
                        .into(),
                    );
                }
                index.advance(Arc::new(state.clone()), &removal);
                actual.advance(&index);
                expected.advance(&index);
                actual.reset(&index);
                expected.reset(&index);
            }
        }
    }
}

#[test]
fn subscription() {
    let particle = ["A"; 8].join(".");
    let source = format!(
        "{},{} [{particle}] Done",
        vec![particle.clone(); 40].join(","),
        vec!["X"; 40].join(",")
    );
    let program = Program::new(&crate::lowering::parse(&source).unwrap());
    let input = Input::new(&program.rule[0].input);
    let mut state = crate::state::State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let store = Arc::new(Store::new(65536));
    let previous = Search::planned(Request {
        input: &input,
        index: &index,
        frame: 0,
        owner: 0,
        store: &store,
    });
    for _ in 0..40 {
        state.world.remove(0);
    }
    index.advance(Arc::new(state), &(0..40).collect());
    let actual = Search::planned(Request {
        input: &input,
        index: &index,
        frame: 0,
        owner: 0,
        store: &store,
    });
    let expected = Join::planned(Request {
        input: &input,
        index: &index,
        frame: 0,
        owner: 0,
        store: &store,
    });
    assert_eq!(actual.retained(), expected.retained());
    assert!(!actual.viable());
    assert!(previous.viable());
}
