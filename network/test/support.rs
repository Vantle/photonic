use crate::configuration::Configuration;
use crate::input::{Input, Pointer, Sample};
use random::Generator;

pub fn configuration() -> Configuration {
    Configuration {
        field: vec![7, 3, 5],
        width: 8,
        depth: 2,
        head: 2,
        hidden: 12,
        unary: 3,
        binary: 2,
        key: 4,
        judge: 2,
    }
}

pub fn input(generator: &mut Generator, configuration: &Configuration) -> Input {
    let length = 3 + generator.below(5);
    let feature = (0..length)
        .flat_map(|_| {
            configuration
                .field
                .iter()
                .map(|&cardinality| generator.below(cardinality) as u16)
                .collect::<Vec<_>>()
        })
        .collect();
    let pointer = (0..2 + generator.below(6))
        .map(|_| {
            if generator.chance(0.5) {
                Pointer::Unary {
                    head: generator.below(configuration.unary) as u16,
                    token: generator.below(length) as u32,
                }
            } else {
                Pointer::Binary {
                    head: generator.below(configuration.binary) as u16,
                    left: generator.below(length) as u32,
                    right: generator.below(length) as u32,
                }
            }
        })
        .collect();
    Input { feature, pointer }
}

pub fn sample(generator: &mut Generator, configuration: &Configuration) -> Sample {
    let input = input(generator, configuration);
    if generator.chance(0.3) {
        return Sample {
            input,
            policy: Vec::new(),
            value: None,
            judge: Some(generator.normal() as f32),
        };
    }
    let policy = input
        .pointer
        .iter()
        .map(|_| generator.uniform() as f32)
        .collect();
    let value = Some(generator.normal() as f32);
    let judge = generator.chance(0.5).then(|| generator.normal() as f32);
    Sample {
        input,
        policy,
        value,
        judge,
    }
}
