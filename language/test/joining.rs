use crate::index::Index;
use crate::joining::Join;
use crate::program::Symbol;
use crate::slot::Slot;
use crate::state::{Frame, State, Token, World};
use crate::term::Term;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::task::Poll;

fn collect(
    mut step: impl FnMut() -> Poll<Option<Vec<Slot>>>,
) -> BTreeSet<Vec<(usize, Vec<usize>, usize)>> {
    let mut result = BTreeSet::new();
    for _ in 0..100_000 {
        match step() {
            Poll::Ready(Some(value)) => {
                result.insert(
                    value
                        .into_iter()
                        .map(|slot| (slot.world, slot.token, slot.position))
                        .collect(),
                );
            }
            Poll::Ready(None) => return result,
            Poll::Pending => {}
        }
    }
    panic!("join did not finish");
}

#[test]
fn differential() {
    let mut seed = 42u64;
    let mut next = |bound| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as usize % bound
    };
    for _ in 0..256 {
        let mut state = State {
            world: (0..4)
                .map(|world| {
                    World {
                        frame: 0,
                        particle: (0..4)
                            .map(|token| Token {
                                id: world * 4 + token,
                                value: if next(3) == 0 {
                                    Symbol::Rule(next(2))
                                } else {
                                    Symbol::Atom(next(3))
                                },
                                capture: Some(next(2)),
                            })
                            .collect(),
                    }
                    .into()
                })
                .collect(),
            frame: vec![
                Frame {
                    scope: 0,
                    parent: None,
                    lexical: None,
                    held: Vec::new(),
                }
                .into(),
            ]
            .into(),
        };
        let pattern = Arc::new(
            (0..next(3) + 1)
                .map(|_| {
                    (0..next(3))
                        .map(|_| {
                            Term::new(
                                if next(3) == 0 {
                                    Symbol::Rule(next(2))
                                } else {
                                    Symbol::Atom(next(3))
                                },
                                Some(next(2)),
                            )
                        })
                        .collect()
                })
                .collect::<Vec<Vec<_>>>(),
        );
        let mut index = Index::new(Arc::new(state.clone()));
        let mut join = Join::new(pattern.clone(), &index, 0);
        for _ in 0..4 {
            let mut reference = crate::search::Search::new(
                (*pattern).clone(),
                Arc::new(Index::new(Arc::new(state.clone()))),
                0,
            );
            assert_eq!(collect(|| join.step(&index)), collect(|| reference.step()));
            join.reset(&index);
            let removed = next(state.world.len());
            let mut world = (*state.world.remove(removed)).clone();
            for token in &mut world.particle {
                token.value = Symbol::Atom(next(3));
                token.capture = None;
            }
            state.world.push(world.into());
            index.advance(Arc::new(state.clone()), &crate::basis::Set::single(removed));
            join.advance(&index);
        }
    }
}

#[test]
fn width() {
    let program = crate::program::Program::new(
        crate::lowering::parse("A,B,C,D,E,F,G,H [A,B,C,D,E,F,G,H] End").unwrap(),
    );
    let state = Arc::new(State::initial(&program));
    let index = Index::new(state.clone());
    let pattern = crate::plan::Input::new(&program.rule[0].input).pattern(0);
    let mut join = Join::new(pattern.clone(), &index, 0);
    let mut reference =
        crate::search::Search::new((*pattern).clone(), Arc::new(Index::new(state)), 0);
    let actual = collect(|| join.step(&index));
    assert_eq!(actual.len(), 1);
    assert_eq!(actual, collect(|| reference.step()));
}
