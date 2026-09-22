use crate::change::Change;
use crate::dispatch::Network;
use crate::index::Index;
use crate::program::Program;
use crate::state::State;
use serde::Serialize;
use std::sync::Arc;
use std::task::Poll;
use std::time::Instant;

#[derive(Serialize)]
pub struct Measurement {
    pub initialization: f64,
    pub execution: f64,
    pub retained: usize,
    pub preparation: usize,
    pub reuse: usize,
}

pub fn run(width: usize, length: usize) -> Measurement {
    let start = Instant::now();
    let program = Program::new(crate::lowering::parse("X [A] B").unwrap());
    let mut state = State::initial(&program);
    let initial = state.world[0].clone();
    state.frame = vec![state.frame[0].clone(); width + 1].into();
    state.world = Vec::new().into();
    for frame in 0..=width {
        if frame > 0 && frame < width {
            Arc::make_mut(&mut state.frame[frame]).lexical = Some(frame - 1);
        }
        let mut world = (*initial).clone();
        world.frame = frame;
        world.particle[0].id = frame;
        state.world.push(world.into());
    }
    let mut index = Index::new(Arc::new(state.clone()));
    let mut network = Network::new(&program, &index);
    let change = Change {
        world: Default::default(),
        insertion: state.world.len()..state.world.len(),
        frame: vec![width],
    };
    let initialization = start.elapsed().as_secs_f64();
    let start = Instant::now();
    for iteration in 0..length {
        Arc::make_mut(&mut state.frame[width]).lexical = (iteration % 2 == 0).then_some(0);
        index.update(Arc::new(state.clone()), &change);
        network.advance(&index);
        assert!(matches!(network.next(&index), Poll::Ready(None)));
    }
    Measurement {
        initialization,
        execution: start.elapsed().as_secs_f64(),
        retained: network.retained(),
        preparation: network.preparation,
        reuse: network.reuse,
    }
}
