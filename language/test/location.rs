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
                    let world = &index.state.world[slot.location.world().unwrap()];
                    assert!(
                        slot.token
                            .iter()
                            .all(|id| { world.particle.iter().any(|token| token.id == *id) })
                    );
                    selection.push(slot.location.world().unwrap());
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
                "{particle},B,{},{particle},B,C, [{particle},B,C] Done",
                vec!["X"; length].join(",")
            );
            let program = Program::new(&frontend::lowering::parse(&source).unwrap());
            let input = Input::new(&program.rule[0].input);
            let mut state = State::initial(&program);
            let mut index = Index::new(Arc::new(state.clone()));
            let store = Arc::new(Store::new(65536));
            let mut join = Join::planned(Request {
                context: input.context(0),
                index: &index,
                frame: 0,
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
                crate::test::advance(
                    &mut index,
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

#[test]
fn context() {
    use crate::program::Symbol;
    use crate::state::Token;
    let source = format!("{},X,Y, [X] Z", vec!["A"; 40].join(","));
    let program = Program::new(&frontend::lowering::parse(&source).unwrap());
    let atom = |name| Symbol::Atom(program.atom.get_index_of(name).unwrap());
    let input = Input::new(&[
        std::iter::once(atom("A"))
            .chain(std::iter::repeat_n(Symbol::Rule(0), 8))
            .collect(),
        vec![atom("X")],
        vec![atom("Y")],
    ]);
    let mut state = State::initial(&program);
    let mut frame = (*state.frame[0]).clone();
    frame.particle.clear();
    frame.parent = Some(0);
    frame.lexical = Some(0);
    state.frame.push(frame.into());
    Arc::make_mut(&mut state.frame[0]).particle = (0..8)
        .map(|position| Token {
            id: 100 + position,
            value: Symbol::Rule(0),
            capture: Some(0),
        })
        .collect();
    let mut index = Index::new(Arc::new(state.clone()));
    let store = Arc::new(Store::new(65536));
    let create = |index: &Index| {
        Join::planned(Request {
            context: input.context(0),
            index,
            frame: 0,
            store: &store,
        })
    };
    let drain = |join: &mut Join, index: &Index| {
        let mut result = Vec::new();
        for _ in 0..10000 {
            match join.step(index) {
                Poll::Ready(Some(binding)) => result.push(
                    binding
                        .into_iter()
                        .map(|slot| (slot.location, slot.position, slot.token))
                        .collect::<Vec<_>>(),
                ),
                Poll::Ready(None) => {
                    result.sort();
                    return result;
                }
                Poll::Pending => {}
            }
        }
        panic!("context join did not complete");
    };
    let mut join = create(&index);
    for iteration in 0..64 {
        let mut fresh = create(&index);
        let expected = drain(&mut fresh, &index);
        assert_eq!(drain(&mut join, &index), expected);
        assert_eq!(join.retained(), join.size());
        join.reset(&index);
        for _ in 0..iteration % 19 {
            let _ = join.step(&index);
        }
        if iteration % 5 == 0 {
            join.evict();
            store.evict();
        }
        Arc::make_mut(&mut state.frame[0]).particle = (0..8 - iteration % 2)
            .map(|position| Token {
                id: 200 + iteration * 8 + position,
                value: Symbol::Rule(0),
                capture: Some(iteration / 2 % 2),
            })
            .collect();
        index.update(
            Arc::new(state.clone()),
            &crate::change::Change {
                world: Default::default(),
                insertion: state.world.len()..state.world.len(),
                frame: vec![0],
            },
        );
        join.advance(&index);
    }
}
