use super::{Join, Request, Store};
use crate::index::Index;
use crate::plan::Input;
use crate::program::Program;
use crate::state::State;
use std::sync::Arc;
use std::task::Poll;

#[test]
fn subscription() {
    let content = vec!["A.B,A.B.B,A.C"; 16].join(",");
    let program = Program::new(
        &crate::lowering::parse(&format!(
            "{content},C [A.B,B] First [B.A.A,C] Second [A.B,A.B] Third"
        ))
        .unwrap(),
    );
    let input = program
        .rule
        .iter()
        .map(|rule| Input::new(&rule.input))
        .collect::<Vec<_>>();
    for capacity in [0, 32, 128, 65536] {
        let store = Arc::new(Store::new(capacity));
        let mut state = State::initial(&program);
        let mut index = Index::new(Arc::new(state.clone()));
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
        for iteration in 0..48 {
            for (query, reference) in query.iter_mut().zip(&mut reference) {
                for _ in 0..iteration {
                    assert_eq!(query.step(&index), reference.step(&index));
                }
                if iteration % 11 == 10 {
                    query.evict();
                }
                loop {
                    let result = query.step(&index);
                    assert_eq!(result, reference.step(&index));
                    assert_eq!(query.retained(), query.size());
                    if result == Poll::Ready(None) {
                        break;
                    }
                }
            }
            if iteration % 11 == 10 {
                store.evict();
            }
            let position = iteration % state.world.len();
            let first = state.world.remove(position);
            let second = state.world.remove(position);
            state.world.push(second);
            state.world.push(first);
            index.advance(
                Arc::new(state.clone()),
                &[position, position + 1].into_iter().collect(),
            );
            for (query, reference) in query.iter_mut().zip(&mut reference) {
                query.advance(&index);
                reference.advance(&index);
            }
            if iteration % 11 == 1 {
                query[0] = Join::planned(Request {
                    input: &input[0],
                    index: &index,
                    frame: 0,
                    owner: 0,
                    store: &store,
                });
                reference[0] = Join::new(input[0].pattern(0), &index, 0);
            }
        }
        drop(query);
        store.evict();
        assert_eq!(store.retained(), 0);
        assert!(store.budget().reserve(capacity));
        store.budget().release(capacity);
    }
}
