use super::{Delivery, Network};
use crate::change::Change;
use crate::index::Index;
use crate::program::Program;
use crate::state::{State, Token, World};
use std::sync::Arc;
use std::task::Poll;

fn drain(network: &mut Network, index: &Index) -> Vec<Delivery> {
    let mut delivery = Vec::new();
    for _ in 0..10_000 {
        match network.next(index) {
            Some(Poll::Ready(value)) => delivery.push(value),
            Some(Poll::Pending) => {}
            None => return delivery,
        }
    }
    panic!("dispatch did not finish");
}

fn advance(network: &mut Network, index: &mut Index, state: State, change: Change) {
    let previous = index.state.clone();
    index.update(Arc::new(state), &change);
    network.advance(index, &previous, &change);
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
