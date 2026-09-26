use crate::change::Change;
use crate::dispatch::Network;
use crate::index::Index;
use crate::program::{Program, Symbol};
use crate::state::{State, Token, World};
use serde::Serialize;
use std::sync::Arc;
use std::task::Poll;

#[derive(Serialize)]
pub struct Observation {
    step: Vec<Step>,
    eviction: usize,
    work: usize,
    binding: usize,
    retained: usize,
    preparation: usize,
    reuse: usize,
}

#[derive(Serialize)]
enum Step {
    Pending,
    Complete,
    Binding {
        rule: usize,
        frame: usize,
        owner: usize,
        read: Option<crate::place::Place>,
        selection: Vec<Selection>,
    },
}

#[derive(Serialize)]
struct Selection {
    location: crate::location::Location,
    position: usize,
    token: Vec<usize>,
}

pub fn run() -> Vec<Observation> {
    let mut source = String::from(
        "A.B, [A] B, [A,A] C, [A.B] D, [] E,
",
    );
    for width in 2..=12 {
        source.push_str(&format!(
            "[{}] End{width},
",
            vec!["A"; width].join(".")
        ));
    }
    let program = Program::new(&frontend::lowering::parse(&source).unwrap());
    let mut state = State::initial(&program);
    state.frame = vec![state.frame[0].clone(); 8].into();
    for frame in 1..8 {
        Arc::make_mut(&mut state.frame[frame]).lexical = Some(frame / 2);
    }
    let mut index = Index::new(Arc::new(state.clone()));
    let mut network = Network::new(&program, &index);
    let mut seed = 43u64;
    let mut next = |bound| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as usize % bound
    };
    let mut work = 0;
    let mut binding = 0;
    let mut observation = Vec::new();
    for iteration in 0..128 {
        let budget = if iteration % 8 == 0 {
            10000
        } else {
            iteration % 31
        };
        let mut complete = false;
        let mut step = Vec::new();
        for _ in 0..budget {
            work += 1;
            match network.next(&index) {
                Poll::Pending => step.push(Step::Pending),
                Poll::Ready(None) => {
                    step.push(Step::Complete);
                    complete = true;
                    break;
                }
                Poll::Ready(Some(delivery)) => {
                    binding += 1;
                    step.push(Step::Binding {
                        rule: delivery.rule,
                        frame: delivery.frame,
                        owner: delivery.owner,
                        read: delivery.read.map(|read| read.place(&index)),
                        selection: delivery
                            .selection
                            .into_iter()
                            .map(|slot| Selection {
                                location: slot.location,
                                position: slot.position,
                                token: slot.token,
                            })
                            .collect(),
                    });
                }
            }
        }
        if iteration % 8 == 0 {
            assert!(complete);
        }
        let eviction = if iteration % 11 == 0 {
            network.evict()
        } else {
            0
        };
        observation.push(Observation {
            step,
            eviction,
            work,
            binding,
            retained: network.retained(),
            preparation: network.preparation(),
            reuse: network.reuse(),
        });
        let removal = if state.world.len() == 0 {
            Default::default()
        } else {
            let position = next(state.world.len());
            state.world.remove(position);
            crate::basis::Set::single(position)
        };
        let start = state.world.len();
        for position in 0..next(3).min(12 - start) {
            state.world.push(
                World {
                    frame: next(8),
                    particle: (0..next(4))
                        .map(|token| {
                            let value = if next(3) == 0 {
                                Symbol::Rule(next(program.rule.len()))
                            } else {
                                Symbol::Atom(next(4))
                            };
                            Token {
                                id: 100 + iteration * 16 + position * 4 + token,
                                value,
                                capture: matches!(value, Symbol::Rule(_)).then(|| next(8)),
                            }
                        })
                        .collect(),
                }
                .into(),
            );
        }
        let frame = if iteration % 3 == 0 {
            let frame = next(7) + 1;
            Arc::make_mut(&mut state.frame[frame]).lexical = (next(2) == 0).then(|| next(frame));
            vec![frame]
        } else {
            Vec::new()
        };
        let change = Change {
            world: removal,
            insertion: start..state.world.len(),
            frame,
        };
        index.update(Arc::new(state.clone()), &change);
        network.advance(&index);
    }
    observation
}
