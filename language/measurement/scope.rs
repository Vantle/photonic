use crate::change::Change;
use crate::dispatch::Network;
use crate::index::Index;
use crate::program::{Program, Symbol};
use crate::state::{State, Token};
use serde::Serialize;
use std::sync::Arc;
use std::task::Poll;
use std::time::Instant;

#[derive(Serialize)]
pub struct Measurement {
    initialization: f64,
    execution: f64,
    release: f64,
    work: usize,
    delivery: usize,
    fingerprint: u64,
    retained: usize,
    preparation: usize,
    reuse: usize,
}

pub fn run(width: usize, depth: usize, length: usize, stride: usize) -> Measurement {
    let source = (0..width)
        .map(|position| format!("[Value{position}] Result"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut program = Program::new(&crate::lowering::parse(&format!("Seed {source}")).unwrap());
    program.scope = vec![program.scope[0].clone(); depth];
    let mut state = State::initial(&program);
    let initial = state.world[0].clone();
    let mut next = depth;
    state.frame = (0..depth)
        .map(|scope| {
            let mut frame = (*state.frame[0]).clone();
            frame.scope = scope;
            frame.particle = program.scope[scope]
                .rule
                .iter()
                .map(|&rule| Token::new(Symbol::Rule(rule), scope, &mut next))
                .collect();
            Arc::new(frame)
        })
        .collect();
    state.world = (0..depth)
        .map(|position| {
            let mut world = (*initial).clone();
            world.frame = (position + 1) % depth;
            world.particle[0].id = position;
            Arc::new(world)
        })
        .collect();
    let symbol = (0..width)
        .step_by(stride)
        .map(|position| {
            Symbol::Atom(
                program
                    .atom
                    .get_index_of(&format!("Value{position}"))
                    .unwrap(),
            )
        })
        .collect::<Vec<_>>();
    let position = state.world.len() - 1;
    let change = Change {
        world: crate::basis::Set::single(position),
        insertion: position..position + 1,
        frame: Vec::new(),
    };
    let start = Instant::now();
    let mut index = Index::new(Arc::new(state.clone()));
    let mut network = Network::new(&program, &index);
    let initialization = start.elapsed().as_secs_f64();
    let start = Instant::now();
    let mut work = 0;
    let mut delivery = 0;
    let mut fingerprint = 0u64;
    for iteration in 0..length {
        let mut world = (*state.world.remove(position)).clone();
        world.particle = if iteration % 2 == 0 {
            symbol
                .iter()
                .enumerate()
                .map(|(offset, &value)| Token {
                    id: next + iteration * width + offset,
                    value,
                    capture: None,
                })
                .collect()
        } else {
            Vec::new()
        };
        state.world.push(world.into());
        index.update(Arc::new(state.clone()), &change);
        network.advance(&index);
        loop {
            work += 1;
            match network.next(&index) {
                Poll::Ready(None) => break,
                Poll::Pending => {}
                Poll::Ready(Some(value)) => {
                    delivery += 1;
                    for value in [value.rule, value.frame, value.owner].into_iter().chain(
                        value.selection.into_iter().flat_map(|slot| {
                            [
                                match slot.location {
                                    crate::location::Location::World(world) => world * 2,
                                    crate::location::Location::Context(frame) => frame * 2 + 1,
                                },
                                slot.position,
                            ]
                            .into_iter()
                            .chain(slot.token)
                        }),
                    ) {
                        fingerprint = fingerprint
                            .wrapping_mul(1099511628211)
                            .wrapping_add(value as u64);
                    }
                }
            }
        }
    }
    let execution = start.elapsed().as_secs_f64();
    let retained = network.retained();
    let preparation = network.preparation;
    let reuse = network.reuse;
    let start = Instant::now();
    drop(network);
    drop(index);
    let release = start.elapsed().as_secs_f64();
    Measurement {
        initialization,
        execution,
        release,
        work,
        delivery,
        fingerprint,
        retained,
        preparation,
        reuse,
    }
}
