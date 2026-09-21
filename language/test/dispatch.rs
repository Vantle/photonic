use super::{Delivery, Network};
use crate::change::Change;
use crate::index::Index;
use crate::program::Program;
use crate::state::{State, Token, World};
use std::sync::Arc;
use std::task::Poll;

fn accounting(network: &Network) {
    assert_eq!(
        network.storage,
        network
            .store
            .iter()
            .map(super::entry::Entry::retained)
            .sum::<usize>()
    );
    assert_eq!(network.entry.len(), network.count.iter().sum::<usize>());
    if let Some(ready) = &network.ready {
        assert_eq!(
            ready.len(),
            network.store.iter().filter(|entry| entry.viable()).count()
        );
        for (&key, &position) in ready {
            assert_eq!(network.entry.get(&key), Some(&position));
            assert!(network.store[position].viable());
        }
    }
}

#[test]
fn batching() {
    let particle = ["A"; 8].join(".");
    let program = Program::new(crate::lowering::parse(&format!(
        "{particle}.A,{particle}.A,B,B,C [{particle},B] Left [{particle},B] Right [{particle},C] Third [B,B] Pair"
    )).unwrap());
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let mut actual = Network::new(&program, &index);
    let mut expected = Network::new(&program, &index);
    let observe = |result: Poll<Option<Delivery>>| {
        result.map(|delivery| {
            delivery.map(|delivery| {
                (
                    delivery.rule,
                    delivery.frame,
                    delivery.owner,
                    delivery.read.map(|read| (read.site, read.resource)),
                    delivery.selection,
                )
            })
        })
    };
    let mut batched = 0;
    for iteration in 0..32 {
        let mut complete = false;
        for step in 0..10_000 {
            let retained = actual.retained();
            let skipped = actual.skip([0, 1, 2, 3, 7, 31, 128][step % 7]);
            batched += skipped;
            assert_eq!(actual.retained(), retained);
            for _ in 0..skipped {
                assert!(matches!(expected.next(&index), Poll::Pending));
            }
            assert_eq!(actual.agenda, expected.agenda);
            accounting(&actual);
            assert_eq!(actual.retained(), expected.retained());
            if iteration % 3 == 0 && step == 17 {
                assert_eq!(actual.evict(), expected.evict());
            }
            let result = observe(actual.next(&index));
            assert_eq!(result, observe(expected.next(&index)));
            if matches!(result, Poll::Ready(None)) {
                complete = true;
                break;
            }
        }
        assert!(complete);
        if iteration % 4 == 3 {
            let previous = index.state.clone();
            let mut world = (*state.world.remove(0)).clone();
            for token in &mut world.particle {
                token.id += 1000;
            }
            state.world.push(world.into());
            let change = Change {
                world: crate::basis::Set::single(0),
                insertion: state.world.len() - 1..state.world.len(),
                frame: Vec::new(),
            };
            index.update(Arc::new(state.clone()), &change);
            actual.advance(&index, &previous, &change);
            expected.advance(&index, &previous, &change);
        } else {
            actual.reset(&index);
            expected.reset(&index);
        }
    }
    assert!(batched > 0);
}

#[test]
fn promotion() {
    let mut source = (0..64)
        .map(|index| format!("Idle{index},"))
        .collect::<String>();
    source.push_str("Stage\n");
    for index in 0..64 {
        source.push_str(&format!("[Idle{index},Idle{index}] Never{index}\n"));
    }
    source.push_str("[Stage] End");
    let program = Program::new(crate::lowering::parse(&source).unwrap());
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let mut network = Network::new(&program, &index);
    assert!(network.ready.is_some());
    assert_eq!(drain(&mut network, &index).len(), 1);
    state.frame.push(state.frame[0].clone());
    Arc::make_mut(&mut state.frame[1]).lexical = Some(0);
    let symbol = crate::program::Symbol::Atom(program.atom.get_index_of("Idle0").unwrap());
    let mut world = (**state
        .world
        .iter()
        .find(|world| world.particle[0].value == symbol)
        .unwrap())
    .clone();
    world.frame = 1;
    state.world.push(world.into());
    advance(
        &mut network,
        &mut index,
        state.clone(),
        Change {
            world: Default::default(),
            insertion: 65..66,
            frame: vec![1],
        },
    );
    assert_eq!(drain(&mut network, &index).len(), 1);
    let symbol = crate::program::Symbol::Atom(program.atom.get_index_of("Stage").unwrap());
    let retained = state
        .world
        .iter()
        .position(|world| world.particle[0].value == symbol)
        .unwrap();
    let removed = (0..state.world.len())
        .filter(|&world| world != retained)
        .collect();
    state.world = vec![state.world[retained].clone()].into();
    state.frame.truncate(1);
    advance(
        &mut network,
        &mut index,
        state,
        Change {
            world: removed,
            insertion: 1..1,
            frame: vec![1],
        },
    );
    assert_eq!(drain(&mut network, &index).len(), 1);
    assert_eq!(network.entry.len(), 1);
}

#[test]
fn mutation() {
    use crate::program::Symbol;
    let program = Program::new(crate::lowering::parse("A [A] B [A,A] C [A.B] D [] E").unwrap());
    let mut state = State::initial(&program);
    state.frame.push(state.frame[0].clone());
    Arc::make_mut(&mut state.frame[1]).lexical = Some(0);
    let mut index = Index::new(Arc::new(state.clone()));
    let mut network = Network::new(&program, &index);
    let normalize = |delivery: Vec<Delivery>, index: &Index| {
        let mut value = delivery
            .into_iter()
            .map(|delivery| {
                (
                    delivery.rule,
                    delivery.frame,
                    delivery.owner,
                    delivery
                        .read
                        .map(|read| (index.world(read.site), read.resource)),
                    delivery
                        .selection
                        .into_iter()
                        .map(|slot| (slot.world, slot.position, slot.token))
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        value.sort_unstable();
        value
    };
    let mut seed = 17u64;
    let mut next = |bound| {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        (seed >> 32) as usize % bound
    };
    for iteration in 0..256 {
        let fresh = Index::new(Arc::new(state.clone()));
        let mut reference = Network::new(&program, &fresh);
        let expected = normalize(drain(&mut reference, &fresh), &fresh);
        for _ in 0..3 {
            assert_eq!(normalize(drain(&mut network, &index), &index), expected);
            network.reset(&index);
        }
        let removed = if state.world.is_empty() {
            crate::basis::Set::default()
        } else {
            let removed = next(state.world.len());
            state.world.remove(removed);
            crate::basis::Set::single(removed)
        };
        let start = state.world.len();
        for world in 0..next(3).min(4 - start) {
            state.world.push(
                World {
                    frame: next(2),
                    particle: (0..next(4))
                        .map(|token| {
                            let value = if next(3) == 0 {
                                Symbol::Rule(next(program.rule.len()))
                            } else {
                                Symbol::Atom(next(program.atom.len()))
                            };
                            Token {
                                id: 100 + iteration * 8 + world * 4 + token,
                                value,
                                capture: matches!(value, Symbol::Rule(_)).then(|| next(2)),
                            }
                        })
                        .collect(),
                }
                .into(),
            );
        }
        let frame = if iteration % 7 == 0 {
            Arc::make_mut(&mut state.frame[1]).lexical = (next(2) == 0).then_some(0);
            vec![1]
        } else {
            Vec::new()
        };
        let end = state.world.len();
        advance(
            &mut network,
            &mut index,
            state.clone(),
            Change {
                world: removed,
                insertion: start..end,
                frame,
            },
        );
    }
}

fn drain(network: &mut Network, index: &Index) -> Vec<Delivery> {
    let mut delivery = Vec::new();
    for _ in 0..10_000 {
        accounting(network);
        let next = network.next(index);
        accounting(network);
        match next {
            Poll::Ready(Some(value)) => delivery.push(value),
            Poll::Pending => {}
            Poll::Ready(None) => return delivery,
        }
    }
    panic!("dispatch did not finish");
}

fn advance(network: &mut Network, index: &mut Index, state: State, change: Change) {
    let previous = index.state.clone();
    index.update(Arc::new(state), &change);
    network.advance(index, &previous, &change);
    accounting(network);
}

#[test]
fn sharing() {
    let program = Program::new(crate::lowering::parse("X,A [A] B [A] C").unwrap());
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let mut network = Network::new(&program, &index);
    assert_eq!(network.entry.len(), 1);
    assert_eq!(network.preparation, 1);
    assert_eq!(drain(&mut network, &index).len(), 2);
    let matched = program.rule[0].input[0][0];
    let removed = state
        .world
        .iter()
        .position(|world| world.particle[0].value != matched)
        .unwrap();
    state.world.remove(removed);
    advance(
        &mut network,
        &mut index,
        state.clone(),
        Change {
            world: [removed].into_iter().collect(),
            insertion: 1..1,
            frame: Vec::new(),
        },
    );
    assert_eq!(network.preparation, 1);
    assert_eq!(network.reuse, 1);
    assert_eq!(drain(&mut network, &index).len(), 2);
    Arc::make_mut(&mut state.world[0]).particle[0].id = 100;
    advance(
        &mut network,
        &mut index,
        state,
        Change {
            world: [0].into_iter().collect(),
            insertion: 0..1,
            frame: Vec::new(),
        },
    );
    let delivery = drain(&mut network, &index);
    assert_eq!(delivery.len(), 2);
    assert!(
        delivery
            .iter()
            .all(|delivery| delivery.selection[0].token == [100])
    );
}

#[test]
fn multiplicity() {
    let program = Program::new(crate::lowering::parse("A [A,A] B").unwrap());
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let mut network = Network::new(&program, &index);
    assert!(drain(&mut network, &index).is_empty());
    state.world.push(
        World {
            frame: 0,
            particle: vec![Token {
                id: 100,
                value: program.rule[0].input[0][0],
                capture: None,
            }],
        }
        .into(),
    );
    advance(
        &mut network,
        &mut index,
        state,
        Change {
            world: Default::default(),
            insertion: 1..2,
            frame: Vec::new(),
        },
    );
    let delivery = drain(&mut network, &index);
    assert_eq!(delivery.len(), 1);
    assert_ne!(
        delivery[0].selection[0].world,
        delivery[0].selection[1].world
    );
}

#[test]
fn empty() {
    let program = Program::new(crate::lowering::parse("[] A").unwrap());
    let mut state = State::initial(&program);
    state.world.clear();
    state.world.push(
        World {
            frame: 0,
            particle: Vec::new(),
        }
        .into(),
    );
    let mut index = Index::new(Arc::new(state.clone()));
    let mut network = Network::new(&program, &index);
    assert_eq!(drain(&mut network, &index).len(), 1);
    state.world.clear();
    advance(
        &mut network,
        &mut index,
        state,
        Change {
            world: [0].into_iter().collect(),
            insertion: 0..0,
            frame: Vec::new(),
        },
    );
    assert!(drain(&mut network, &index).is_empty());
}

#[test]
fn occupancy() {
    let program = Program::new(crate::lowering::parse("A [A] B").unwrap());
    let mut state = State::initial(&program);
    state.frame.push(state.frame[0].clone());
    Arc::make_mut(&mut state.frame[1]).lexical = Some(0);
    let mut index = Index::new(Arc::new(state.clone()));
    let mut network = Network::new(&program, &index);
    assert_eq!(drain(&mut network, &index).len(), 1);
    let mut world = (*state.world[0]).clone();
    world.frame = 1;
    state.world.push(world.into());
    advance(
        &mut network,
        &mut index,
        state,
        Change {
            world: Default::default(),
            insertion: 1..2,
            frame: Vec::new(),
        },
    );
    let delivery = drain(&mut network, &index);
    assert_eq!(delivery.len(), 3);
    assert_eq!(network.entry.len(), 2);
    assert_eq!(
        delivery
            .iter()
            .filter(|delivery| delivery.frame == 1)
            .count(),
        2
    );
}

#[test]
fn capture() {
    use crate::program::Symbol;
    let mut program = Program::new(crate::lowering::parse("A [A] B [B] C").unwrap());
    program.rule[0].input = vec![vec![Symbol::Rule(1)]];
    let mut state = State::initial(&program);
    state.frame.push(state.frame[0].clone());
    Arc::make_mut(&mut state.frame[0]).lexical = Some(1);
    state.world = vec![
        World {
            frame: 0,
            particle: vec![Token {
                id: 0,
                value: Symbol::Rule(1),
                capture: Some(0),
            }],
        }
        .into(),
    ]
    .into();
    let mut index = Index::new(Arc::new(state.clone()));
    let mut network = Network::new(&program, &index);
    let delivery = drain(&mut network, &index);
    assert_eq!(delivery.len(), 1);
    assert_eq!(delivery[0].owner, 0);
    Arc::make_mut(&mut state.world[0]).particle[0].capture = Some(1);
    advance(
        &mut network,
        &mut index,
        state,
        Change {
            world: [0].into_iter().collect(),
            insertion: 0..1,
            frame: Vec::new(),
        },
    );
    let delivery = drain(&mut network, &index);
    assert_eq!(delivery.len(), 1);
    assert_eq!(delivery[0].owner, 1);
}

#[test]
fn ancestry() {
    let program = Program::new(crate::lowering::parse("A").unwrap());
    let initial = State::initial(&program);
    let mut seed = 71u64;
    for width in [1, 2, 8, 33, 128] {
        for iteration in 0..32 {
            let mut order = (0..width).collect::<Vec<_>>();
            for position in 0..width {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                order.swap(position, (seed >> 32) as usize % width);
            }
            let mut state = initial.clone();
            state.frame = vec![initial.frame[0].clone(); width].into();
            state.world.clear();
            for (position, &frame) in order.iter().enumerate() {
                if position > 0 {
                    Arc::make_mut(&mut state.frame[frame]).lexical = Some(order[position / 2]);
                }
                if (position + iteration) % 3 != 0 {
                    let mut world = (*initial.world[0]).clone();
                    world.frame = frame;
                    world.particle[0].id = frame;
                    state.world.push(world.into());
                }
            }
            let mut changed = order
                .iter()
                .copied()
                .filter(|frame| (frame + iteration) % 7 == 0)
                .chain([width + 1])
                .collect::<Vec<_>>();
            changed.sort_unstable();
            let index = Index::new(Arc::new(state));
            let expected = index
                .frame()
                .filter(|&frame| {
                    let mut owner = Some(frame);
                    while let Some(current) = owner {
                        if changed.binary_search(&current).is_ok() {
                            return true;
                        }
                        owner = index.state.frame[current].lexical;
                    }
                    false
                })
                .collect::<Vec<_>>();
            assert_eq!(
                super::context::select(&index, &changed).as_slice(),
                expected
            );
        }
    }
}

#[test]
fn activation() {
    let program = Program::new(crate::lowering::parse("A [A.B] C").unwrap());
    let mut state = State::initial(&program);
    let mut index = Index::new(Arc::new(state.clone()));
    let mut network = Network::new(&program, &index);
    for iteration in 0..128 {
        Arc::make_mut(&mut state.world[0]).particle[0] = Token {
            id: iteration + 1,
            value: program.rule[0].input[0][(iteration + 1) % 2],
            capture: None,
        };
        advance(
            &mut network,
            &mut index,
            state.clone(),
            Change {
                world: crate::basis::Set::single(0),
                insertion: 0..1,
                frame: Vec::new(),
            },
        );
        assert!(network.altered.is_empty());
        assert!(network.enabled.is_empty());
        assert_eq!(network.preparation, 0);
        assert!(matches!(network.next(&index), Poll::Ready(None)));
    }
}

#[test]
fn dormancy() {
    let program = Program::new(crate::lowering::parse("A,B [A,B] C").unwrap());
    let mut state = State::initial(&program);
    state.frame.push(state.frame[0].clone());
    Arc::make_mut(&mut state.frame[1]).lexical = Some(0);
    Arc::make_mut(&mut state.world[1]).frame = 1;
    let mut index = Index::new(Arc::new(state.clone()));
    let mut network = Network::new(&program, &index);
    assert_eq!(network.entry.len(), 2);
    assert!(drain(&mut network, &index).is_empty());
    state.world.remove(0);
    advance(
        &mut network,
        &mut index,
        state.clone(),
        Change {
            world: crate::basis::Set::single(0),
            insertion: 1..1,
            frame: Vec::new(),
        },
    );
    assert_eq!(network.entry.len(), 1);
    Arc::make_mut(&mut state.world[0]).particle[0] = Token {
        id: 100,
        value: program.rule[0].input[0][0],
        capture: None,
    };
    advance(
        &mut network,
        &mut index,
        state.clone(),
        Change {
            world: crate::basis::Set::single(0),
            insertion: 0..1,
            frame: Vec::new(),
        },
    );
    assert!(network.altered.is_empty());
    assert_eq!(network.entry.len(), 1);
    assert!(drain(&mut network, &index).is_empty());
    state.world.push(
        World {
            frame: 1,
            particle: vec![Token {
                id: 200,
                value: program.rule[0].input[1][0],
                capture: None,
            }],
        }
        .into(),
    );
    advance(
        &mut network,
        &mut index,
        state,
        Change {
            world: Default::default(),
            insertion: 1..2,
            frame: Vec::new(),
        },
    );
    let delivery = drain(&mut network, &index);
    assert_eq!(delivery.len(), 2);
    for delivery in delivery {
        assert_eq!(delivery.frame, 1);
        assert_eq!(delivery.selection[0].token, [100]);
        assert_eq!(delivery.selection[1].token, [200]);
    }
}
