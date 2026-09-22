use super::{Join, Request, Store};
use crate::index::Index;
use crate::plan::Input;
use crate::program::Program;
use crate::state::State;
use std::sync::Arc;
use std::task::Poll;

fn collect(join: &mut Join, index: &Index) -> Vec<Vec<usize>> {
    let mut result = Vec::new();
    for _ in 0..10000 {
        match join.step(index) {
            Poll::Ready(Some(binding)) => {
                let mut selection = Vec::new();
                for (position, slot) in binding.iter().enumerate() {
                    assert_eq!(slot.position, position);
                    let world = &index.state.world[slot.world];
                    assert!(
                        slot.token
                            .iter()
                            .all(|id| { world.particle.iter().any(|token| token.id == *id) })
                    );
                    selection.push(slot.world);
                }
                result.push(selection);
            }
            Poll::Ready(None) => {
                result.sort();
                return result;
            }
            Poll::Pending => {}
        }
    }
    panic!("join did not complete");
}

#[test]
fn location() {
    for width in [1, 8] {
        for length in [1, 65] {
            let particle = vec!["A"; width].join(".");
            let source = format!(
                "{particle},B,{},{particle},B,C [{particle},B,C] Done",
                vec!["X"; length].join(",")
            );
            let program = Program::new(crate::lowering::parse(&source).unwrap());
            let input = Input::new(&program.rule[0].input);
            let mut state = State::initial(&program);
            let mut index = Index::new(Arc::new(state.clone()));
            let store = Arc::new(Store::new(65536));
            let mut join = Join::planned(Request {
                input: &input,
                index: &index,
                frame: 0,
                owner: 0,
                store: &store,
            });
            let mut displaced = false;
            for iteration in 0..96 {
                let mut expected = vec![Vec::new()];
                for pattern in &program.rule[0].input {
                    expected =
                        expected
                            .into_iter()
                            .flat_map(|selection| {
                                state.world.iter().enumerate().filter_map(
                                    move |(position, world)| {
                                        if selection.contains(&position)
                                            || !pattern.iter().all(|symbol| {
                                                world
                                                    .particle
                                                    .iter()
                                                    .any(|token| token.value == *symbol)
                                            })
                                        {
                                            return None;
                                        }
                                        let mut value = selection.clone();
                                        value.push(position);
                                        Some(value)
                                    },
                                )
                            })
                            .collect();
                }
                expected.sort();
                assert_eq!(collect(&mut join, &index), expected);
                join.reset(&index);
                for _ in 0..iteration % 17 {
                    let _ = join.step(&index);
                }
                if iteration % 3 == 0 {
                    join.evict();
                    store.evict();
                }
                join.reset(&index);
                assert_eq!(collect(&mut join, &index), expected);
                assert_eq!(join.retained(), join.size());
                let position = iteration % state.world.len();
                let world = state.world.remove(position);
                state.world.push(world);
                index.advance(
                    Arc::new(state.clone()),
                    &crate::basis::Set::single(position),
                );
                displaced |= (0..state.world.len()).any(|world| index.site(world) != world);
                join.advance(&index);
            }
            assert!(displaced);
        }
    }
}
