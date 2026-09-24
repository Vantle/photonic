use crate::vocabulary::Vocabulary;
use code::atom::Atom;
use code::configuration::Configuration;
use code::output::Output;
use code::particle::Particle;
use code::program::Program;
use code::rule::Rule;
use code::value::Value;
use random::Generator;

pub fn vocabulary() -> Vocabulary {
    Vocabulary::new(["A", "B", "C", "D", "X"].map(str::to_owned).to_vec())
}

struct Shape {
    nesting: f64,
    depth: usize,
}

fn value(generator: &mut Generator, shape: &Shape, depth: usize) -> Value {
    if depth < shape.depth && generator.chance(shape.nesting) {
        return Value::Rule(Box::new(rule(generator, shape, depth + 1)));
    }
    Value::Atom(Atom(generator.below(5) as u16))
}

fn particle(
    generator: &mut Generator,
    shape: &Shape,
    depth: usize,
    minimum: usize,
    maximum: usize,
) -> Particle {
    let length = minimum + generator.below(maximum - minimum + 1);
    Particle::from(
        (0..length)
            .map(|_| value(generator, shape, depth))
            .collect::<Vec<_>>(),
    )
}

fn rule(generator: &mut Generator, shape: &Shape, depth: usize) -> Rule {
    let input = (0..1 + generator.below(2))
        .map(|_| {
            let minimum = usize::from(!generator.chance(0.1));
            particle(generator, shape, depth, minimum, 2)
        })
        .collect();
    let output = (0..generator.below(3))
        .map(|_| {
            let explicit = particle(generator, shape, depth, 0, 2);
            let body = (depth < shape.depth && generator.chance(shape.nesting)).then(|| {
                (0..1 + generator.below(2))
                    .map(|_| rule(generator, shape, depth + 1))
                    .collect()
            });
            Output::new(explicit, body)
        })
        .collect();
    Rule::new(input, output)
}

fn program(generator: &mut Generator, shape: &Shape) -> Program {
    Program::from(
        (0..1 + generator.below(3))
            .map(|_| rule(generator, shape, 0))
            .collect::<Vec<_>>(),
    )
}

pub fn flat(generator: &mut Generator) -> Program {
    program(
        generator,
        &Shape {
            nesting: 0.0,
            depth: 0,
        },
    )
}

pub fn nested(generator: &mut Generator) -> Program {
    program(
        generator,
        &Shape {
            nesting: 0.2,
            depth: 2,
        },
    )
}

pub fn configuration(generator: &mut Generator) -> Configuration {
    Configuration::from(
        (0..1 + generator.below(3))
            .map(|_| {
                particle(
                    generator,
                    &Shape {
                        nesting: 0.0,
                        depth: 0,
                    },
                    0,
                    1,
                    3,
                )
            })
            .collect::<Vec<_>>(),
    )
}
