use super::maintenance;
use crate::change::Change;
use crate::program::Program;
use crate::state::State;
use std::hint::black_box;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct Configuration {
    pub width: usize,
    pub depth: usize,
    pub count: usize,
    pub replacement: usize,
    pub length: usize,
    pub productive: bool,
    pub alternating: bool,
    pub sample: usize,
    pub scalar: bool,
}

pub fn run(configuration: Configuration) -> Vec<maintenance::Measurement> {
    assert!(configuration.count > 1 && configuration.count <= configuration.width);
    assert!(configuration.replacement > 0 && configuration.replacement <= configuration.count);
    let anchor = (0..configuration.depth)
        .map(|position| format!("P{position}"))
        .collect::<Vec<_>>();
    let prefix = anchor
        .iter()
        .map(|value| format!("{value},{value},"))
        .collect::<String>();
    let input = anchor
        .iter()
        .map(|value| format!("{value},"))
        .collect::<String>();
    let particle = ["A"; 8].join(".");
    let content = vec![particle.as_str(); configuration.count].join(",");
    let companion = vec!["B.C.E"; configuration.width].join(",");
    let pattern = vec!["B"; configuration.width - 1].join(",");
    let constraint = if configuration.productive {
        "B.E"
    } else {
        "B.E.E"
    };
    let program = Program::new(
        &crate::lowering::parse(&format!(
            "{prefix}{content},{companion},C, [{input}{particle},{pattern},{constraint},C] Done"
        ))
        .unwrap(),
    );
    let mut previous = State::initial(&program);
    let state = (0..configuration.length)
        .map(|iteration| {
            let depth = (configuration.alternating && !anchor.is_empty() && iteration % 4 != 0)
                .then(|| (iteration / 4) % anchor.len());
            let symbol = depth.map(|depth| {
                crate::program::Symbol::Atom(program.atom.get_index_of(&anchor[depth]).unwrap())
            });
            let removal = previous
                .world
                .iter()
                .enumerate()
                .filter(|(_, world)| {
                    symbol.map_or(world.particle.len() == 8, |symbol| {
                        world.particle[0].value == symbol
                    })
                })
                .take(if symbol.is_some() {
                    1
                } else {
                    configuration.replacement
                })
                .map(|(position, _)| position)
                .collect::<crate::basis::Set<_>>();
            let insertion = previous.world.len() - removal.len();
            let mut replacement = Vec::new();
            for &position in removal.iter().rev() {
                replacement.push(previous.world.remove(position));
            }
            for world in replacement.into_iter().rev() {
                previous.world.push(world);
            }
            (
                Arc::new(previous.clone()),
                Change {
                    world: removal,
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
    let expected = if configuration.productive {
        configuration.count
            * configuration.width
            * configuration.length
            * 2usize.pow(configuration.depth.try_into().unwrap())
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
