use super::maintenance;
use crate::change::Change;
use crate::program::{Program, Symbol};
use crate::state::State;
use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct Configuration {
    pub width: usize,
    pub length: usize,
    pub sample: usize,
    pub productive: bool,
    pub scalar: bool,
}

pub fn run(configuration: Configuration) -> Vec<maintenance::Measurement> {
    let pattern = ["A"; 8].join(".");
    let particle = vec!["A"; configuration.width].join(".");
    let constraint = if configuration.productive {
        "B.E"
    } else {
        "B.E.E"
    };
    let program = Program::new(crate::lowering::parse(&format!(
        "{particle},{particle},M,M,M,B.C.E,B.C.E,B.C.E,B.C.E,C [{pattern},M,B,B,B,{constraint},C] Done"
    )).unwrap());
    let symbol = Symbol::Atom(program.atom.get_index_of("M").unwrap());
    let mut previous = State::initial(&program);
    let state = (0..configuration.length)
        .map(|_| {
            let position = previous
                .world
                .iter()
                .position(|world| world.particle[0].value == symbol)
                .unwrap();
            let world = previous.world.remove(position);
            let insertion = previous.world.len();
            previous.world.push(world);
            (
                Arc::new(previous.clone()),
                Change {
                    world: crate::basis::Set::single(position),
                    insertion: insertion..previous.world.len(),
                    frame: Vec::new(),
                },
            )
        })
        .collect::<Vec<_>>();
    let evaluate = || {
        if configuration.scalar {
            maintenance::run::<true>(&program, &state)
        } else {
            maintenance::run::<false>(&program, &state)
        }
    };
    let start = Instant::now();
    while start.elapsed() < Duration::from_millis(100) {
        black_box(evaluate());
    }
    let measurement = (0..configuration.sample)
        .map(|_| evaluate())
        .collect::<Vec<_>>();
    let combination = (0..8).fold(1, |count, position| {
        count * (configuration.width - position) / (position + 1)
    });
    let expected = if configuration.productive {
        combination * 24 * configuration.length
    } else {
        0
    };
    assert!(
        measurement
            .iter()
            .all(|measurement| measurement.binding == expected)
    );
    measurement
}
